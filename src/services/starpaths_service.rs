use chrono::{Duration, NaiveDateTime, Utc};
/**
 * @file starpaths_service — business logic for starpath management.
 *
 * @remarks
 * Handles all core operations related to Starpaths:
 *
 *  - Starpath lifecycle (create, read, update, delete)
 *  - Lab composition and ordering within starpaths
 *  - Search and filtering (public + owned)
 *  - User progression tracking
 *
 * Acts as the bridge between:
 *
 *  - Database (PostgreSQL via SQLx)
 *  - HTTP handlers (routes layer)
 *
 * Key characteristics:
 *
 *  - Direct SQL queries (no ORM)
 *  - Visibility rules (public vs private)
 *  - Idempotent progression start
 *  - Ordered labs via `position`
 *  - Explicit error handling with `AppError`
 *
 * @packageDocumentation
 */
use reqwest::{Client, Url};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::starpath::{Starpath, StarpathRow, StarpathVisibility},
    models::starpath_analytics::{
        SessionSummary, StarpathAnalytics, StarpathDropoff, StarpathProgressPoint,
        StarpathProgressSnapshot,
    },
    models::starpath_chapter::{StarpathChapter, StarpathChapterRow},
    models::starpath_input::{
        AddStarpathLabInput, CreateStarpathChapterInput, UpdateStarpathChapterInput,
        UpdateStarpathInput,
    },
    models::starpath_lab::{StarpathLab, StarpathLabRow},
    models::starpath_progress::{LearnerStarpath, StarpathProgress, StarpathProgressRow},
    services::cloud_run_auth,
};

#[derive(Clone)]
pub struct StarpathsService {
    db: PgPool,
    http: Client,
    groups_ms_base: Url,
    sessions_ms_base: Url,
}

fn normalize_language(language: Option<String>) -> Result<String, AppError> {
    let language = language
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "en".to_string());

    if matches!(language.as_str(), "en" | "fr") {
        Ok(language)
    } else {
        Err(AppError::BadRequest(
            "language must be en or fr".to_string(),
        ))
    }
}

impl StarpathsService {
    pub fn new(db: PgPool) -> Self {
        let groups_ms_base = Self::load_groups_ms_base_url();
        let sessions_ms_base = Self::load_sessions_ms_base_url();

        Self {
            db,
            http: Client::new(),
            groups_ms_base,
            sessions_ms_base,
        }
    }

    fn load_groups_ms_base_url() -> Url {
        let raw =
            std::env::var("GROUPS_MS_URL").unwrap_or_else(|_| "http://localhost:3006".to_string());

        let url = Url::parse(&raw).expect("GROUPS_MS_URL must be a valid absolute URL");

        if !matches!(url.scheme(), "http" | "https") {
            panic!("GROUPS_MS_URL must use http or https");
        }

        if url.host_str().is_none() {
            panic!("GROUPS_MS_URL must contain a host");
        }

        if !url.username().is_empty() || url.password().is_some() {
            panic!("GROUPS_MS_URL must not contain credentials");
        }

        if url.query().is_some() || url.fragment().is_some() {
            panic!("GROUPS_MS_URL must not contain query parameters or fragments");
        }

        url
    }

    fn load_sessions_ms_base_url() -> Url {
        let raw = std::env::var("SESSIONS_MS_URL")
            .unwrap_or_else(|_| "http://localhost:3003".to_string());

        let url = Url::parse(&raw).expect("SESSIONS_MS_URL must be a valid absolute URL");

        if !matches!(url.scheme(), "http" | "https") {
            panic!("SESSIONS_MS_URL must use http or https");
        }

        if url.host_str().is_none() {
            panic!("SESSIONS_MS_URL must contain a host");
        }

        if !url.username().is_empty() || url.password().is_some() {
            panic!("SESSIONS_MS_URL must not contain credentials");
        }

        if url.query().is_some() || url.fragment().is_some() {
            panic!("SESSIONS_MS_URL must not contain query parameters or fragments");
        }

        url
    }

    // =========================
    // GET /starpaths
    // =========================
    pub async fn list_starpaths(&self) -> Result<Vec<Starpath>, AppError> {
        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
                s.starpath_id,
                s.creator_id,
                s.name,
                s.description,
                s.language,
                s.difficulty,
                s.visibility,
                s.content_status,
                rs.rating_average,
                COALESCE(rs.rating_count, 0)::BIGINT AS rating_count,
                COALESCE(cc.chapters_count, 0)::BIGINT AS chapters_count,
                COALESCE(lc.labs_count, 0)::BIGINT AS labs_count,
                COALESCE(ps.learners_started, 0)::BIGINT AS learners_started,
                s.created_at
            FROM starpaths s
            LEFT JOIN (
                SELECT
                    starpath_id,
                    AVG(rating)::DOUBLE PRECISION AS rating_average,
                    COUNT(feedback_id)::BIGINT AS rating_count
                FROM starpath_feedbacks
                WHERE deleted_at IS NULL
                GROUP BY starpath_id
            ) rs ON rs.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS chapters_count
                FROM starpath_chapters
                GROUP BY starpath_id
            ) cc ON cc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS labs_count
                FROM starpath_labs
                GROUP BY starpath_id
            ) lc ON lc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS learners_started
                FROM user_starpath_progress
                GROUP BY starpath_id
            ) ps ON ps.starpath_id = s.starpath_id
            WHERE s.visibility = 'public'
              AND s.content_status = 'active'
            ORDER BY s.created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        rows.into_iter().map(Starpath::try_from).collect()
    }

    pub async fn list_starpaths_admin(
        &self,
        query: Option<String>,
        visibility: Option<String>,
        content_status: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<Starpath>, i64), AppError> {
        let query_pattern = query
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{}%", value));
        let visibility_filter = visibility
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty() && value != "all");
        let status_filter = content_status
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty() && value != "all");

        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
                s.starpath_id,
                s.creator_id,
                s.name,
                s.description,
                s.language,
                s.difficulty,
                s.visibility,
                s.content_status,
                rs.rating_average,
                COALESCE(rs.rating_count, 0)::BIGINT AS rating_count,
                COALESCE(cc.chapters_count, 0)::BIGINT AS chapters_count,
                COALESCE(lc.labs_count, 0)::BIGINT AS labs_count,
                COALESCE(ps.learners_started, 0)::BIGINT AS learners_started,
                s.created_at
            FROM starpaths s
            LEFT JOIN (
                SELECT
                    starpath_id,
                    AVG(rating)::DOUBLE PRECISION AS rating_average,
                    COUNT(feedback_id)::BIGINT AS rating_count
                FROM starpath_feedbacks
                WHERE deleted_at IS NULL
                GROUP BY starpath_id
            ) rs ON rs.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS chapters_count
                FROM starpath_chapters
                GROUP BY starpath_id
            ) cc ON cc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS labs_count
                FROM starpath_labs
                GROUP BY starpath_id
            ) lc ON lc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS learners_started
                FROM user_starpath_progress
                GROUP BY starpath_id
            ) ps ON ps.starpath_id = s.starpath_id
            WHERE ($1::TEXT IS NULL OR s.visibility = $1)
              AND ($2::TEXT IS NULL OR s.name ILIKE $2 OR s.description ILIKE $2)
              AND ($3::TEXT IS NULL OR s.content_status = $3)
            ORDER BY s.created_at DESC
            LIMIT $4
            OFFSET $5
            "#,
        )
        .bind(visibility_filter.as_deref())
        .bind(query_pattern.as_deref())
        .bind(status_filter.as_deref())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let total = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM starpaths
            WHERE ($1::TEXT IS NULL OR visibility = $1)
              AND ($2::TEXT IS NULL OR name ILIKE $2 OR description ILIKE $2)
              AND ($3::TEXT IS NULL OR content_status = $3)
            "#,
        )
        .bind(visibility_filter.as_deref())
        .bind(query_pattern.as_deref())
        .bind(status_filter.as_deref())
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        rows.into_iter()
            .map(Starpath::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map(|items| (items, total))
    }

    // =========================
    // GET /starpaths/:id
    // =========================
    pub async fn get_starpath(&self, starpath_id: Uuid) -> Result<Option<Starpath>, AppError> {
        let row = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
                s.starpath_id,
                s.creator_id,
                s.name,
                s.description,
                s.language,
                s.difficulty,
                s.visibility,
                s.content_status,
                rs.rating_average,
                COALESCE(rs.rating_count, 0)::BIGINT AS rating_count,
                COALESCE(cc.chapters_count, 0)::BIGINT AS chapters_count,
                COALESCE(lc.labs_count, 0)::BIGINT AS labs_count,
                COALESCE(ps.learners_started, 0)::BIGINT AS learners_started,
                s.created_at
            FROM starpaths s
            LEFT JOIN (
                SELECT
                    starpath_id,
                    AVG(rating)::DOUBLE PRECISION AS rating_average,
                    COUNT(feedback_id)::BIGINT AS rating_count
                FROM starpath_feedbacks
                WHERE deleted_at IS NULL
                GROUP BY starpath_id
            ) rs ON rs.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS chapters_count
                FROM starpath_chapters
                GROUP BY starpath_id
            ) cc ON cc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS labs_count
                FROM starpath_labs
                GROUP BY starpath_id
            ) lc ON lc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS learners_started
                FROM user_starpath_progress
                GROUP BY starpath_id
            ) ps ON ps.starpath_id = s.starpath_id
            WHERE s.starpath_id = $1
            "#,
        )
        .bind(starpath_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        row.map(Starpath::try_from).transpose()
    }

    // ==========================
    // GET /mystarpaths (creator's starpaths only)
    // ==========================
    pub async fn my_starpaths(&self, creator_id: Uuid) -> Result<Vec<Starpath>, AppError> {
        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
                s.starpath_id,
                s.creator_id,
                s.name,
                s.description,
                s.language,
                s.difficulty,
                s.visibility,
                s.content_status,
                rs.rating_average,
                COALESCE(rs.rating_count, 0)::BIGINT AS rating_count,
                COALESCE(cc.chapters_count, 0)::BIGINT AS chapters_count,
                COALESCE(lc.labs_count, 0)::BIGINT AS labs_count,
                COALESCE(ps.learners_started, 0)::BIGINT AS learners_started,
                s.created_at
            FROM starpaths s
            LEFT JOIN (
                SELECT
                    starpath_id,
                    AVG(rating)::DOUBLE PRECISION AS rating_average,
                    COUNT(feedback_id)::BIGINT AS rating_count
                FROM starpath_feedbacks
                WHERE deleted_at IS NULL
                GROUP BY starpath_id
            ) rs ON rs.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS chapters_count
                FROM starpath_chapters
                GROUP BY starpath_id
            ) cc ON cc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS labs_count
                FROM starpath_labs
                GROUP BY starpath_id
            ) lc ON lc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS learners_started
                FROM user_starpath_progress
                GROUP BY starpath_id
            ) ps ON ps.starpath_id = s.starpath_id
            WHERE s.creator_id = $1
              AND s.content_status = 'active'
            ORDER BY s.created_at DESC
            "#,
        )
        .bind(creator_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        rows.into_iter().map(Starpath::try_from).collect()
    }

    pub async fn learner_starpaths(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<LearnerStarpath>, AppError> {
        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
                s.starpath_id,
                s.creator_id,
                s.name,
                s.description,
                s.language,
                s.difficulty,
                s.visibility,
                s.content_status,
                rs.rating_average,
                COALESCE(rs.rating_count, 0)::BIGINT AS rating_count,
                COALESCE(cc.chapters_count, 0)::BIGINT AS chapters_count,
                COALESCE(lc.labs_count, 0)::BIGINT AS labs_count,
                COALESCE(ps.learners_started, 0)::BIGINT AS learners_started,
                s.created_at
            FROM user_starpath_progress usp
            JOIN starpaths s ON s.starpath_id = usp.starpath_id
            LEFT JOIN (
                SELECT starpath_id, AVG(rating)::DOUBLE PRECISION AS rating_average,
                       COUNT(feedback_id)::BIGINT AS rating_count
                FROM starpath_feedbacks
                WHERE deleted_at IS NULL
                GROUP BY starpath_id
            ) rs ON rs.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS chapters_count
                FROM starpath_chapters
                GROUP BY starpath_id
            ) cc ON cc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS labs_count
                FROM starpath_labs
                GROUP BY starpath_id
            ) lc ON lc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS learners_started
                FROM user_starpath_progress
                GROUP BY starpath_id
            ) ps ON ps.starpath_id = s.starpath_id
            WHERE usp.user_id = $1
            ORDER BY usp.started_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let progress_by_starpath = sqlx::query_as::<_, StarpathProgressRow>(
            r#"
            SELECT user_id, starpath_id, current_position, status, started_at, completed_at
            FROM user_starpath_progress
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .into_iter()
        .map(|row| (row.starpath_id, row))
        .collect::<std::collections::HashMap<_, _>>();

        rows.into_iter()
            .map(|row| {
                let progress = progress_by_starpath.get(&row.starpath_id).ok_or_else(|| {
                    AppError::Internal("Missing learner starpath progress".into())
                })?;

                Ok(LearnerStarpath {
                    starpath: Starpath::try_from(row)?,
                    current_position: progress.current_position,
                    status: progress.status.clone(),
                    started_at: progress.started_at,
                    completed_at: progress.completed_at,
                })
            })
            .collect()
    }

    // ==========================
    // GET /starpaths/search?q=
    // ==========================
    pub async fn search_starpaths(
        &self,
        query: String,
        caller_id: Uuid,
    ) -> Result<Vec<Starpath>, AppError> {
        let pattern = format!("%{}%", query);

        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
                s.starpath_id,
                s.creator_id,
                s.name,
                s.description,
                s.language,
                s.difficulty,
                s.visibility,
                s.content_status,
                rs.rating_average,
                COALESCE(rs.rating_count, 0)::BIGINT AS rating_count,
                COALESCE(cc.chapters_count, 0)::BIGINT AS chapters_count,
                COALESCE(lc.labs_count, 0)::BIGINT AS labs_count,
                COALESCE(ps.learners_started, 0)::BIGINT AS learners_started,
                s.created_at
            FROM starpaths s
            LEFT JOIN (
                SELECT
                    starpath_id,
                    AVG(rating)::DOUBLE PRECISION AS rating_average,
                    COUNT(feedback_id)::BIGINT AS rating_count
                FROM starpath_feedbacks
                WHERE deleted_at IS NULL
                GROUP BY starpath_id
            ) rs ON rs.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS chapters_count
                FROM starpath_chapters
                GROUP BY starpath_id
            ) cc ON cc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS labs_count
                FROM starpath_labs
                GROUP BY starpath_id
            ) lc ON lc.starpath_id = s.starpath_id
            LEFT JOIN (
                SELECT starpath_id, COUNT(*)::BIGINT AS learners_started
                FROM user_starpath_progress
                GROUP BY starpath_id
            ) ps ON ps.starpath_id = s.starpath_id
            WHERE 
                s.name ILIKE $1
                AND s.content_status = 'active'
                AND (
                    s.visibility = 'public'
                    OR s.creator_id = $2
                )
            ORDER BY s.name
            LIMIT 10
            "#,
        )
        .bind(pattern)
        .bind(caller_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        rows.into_iter().map(Starpath::try_from).collect()
    }

    // =========================
    // POST /starpaths (creator only)
    // =========================
    pub async fn create_starpath(
        &self,
        creator_id: Uuid,
        name: String,
        description: Option<String>,
        difficulty: Option<String>,
        visibility: Option<String>,
        language: Option<String>,
    ) -> Result<Starpath, AppError> {
        let visibility = visibility
            .map(|v| {
                if v.len() > 16 {
                    return Err(AppError::BadRequest("visibility too long".into()));
                }
                Ok(v.to_lowercase())
            })
            .transpose()?
            .unwrap_or("private".to_string());
        let language = normalize_language(language)?;

        let row = sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO starpaths (
                creator_id,
                name,
                description,
                language,
                difficulty,
                visibility
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING starpath_id
            "#,
        )
        .bind(creator_id)
        .bind(name)
        .bind(description)
        .bind(language)
        .bind(difficulty)
        .bind(visibility)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.get_starpath(row)
            .await?
            .ok_or_else(|| AppError::Internal("Created starpath could not be reloaded".into()))
    }

    // =========================
    // PUT /starpaths/:id
    // =========================
    pub async fn update_starpath(
        &self,
        starpath_id: Uuid,
        input: UpdateStarpathInput,
    ) -> Result<Option<Starpath>, AppError> {
        let visibility = input
            .visibility
            .map(|v| {
                if v.len() > 16 {
                    return Err(AppError::BadRequest("visibility too long".into()));
                }
                Ok(v.to_lowercase())
            })
            .transpose()?;
        let language = input.language.map(|value| normalize_language(Some(value))).transpose()?;

        let row = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE starpaths
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                language = COALESCE($4, language),
                difficulty = COALESCE($5, difficulty),
                visibility = COALESCE($6, visibility)
            WHERE starpath_id = $1
            RETURNING starpath_id
            "#,
        )
        .bind(starpath_id)
        .bind(input.name)
        .bind(input.description)
        .bind(language)
        .bind(input.difficulty)
        .bind(visibility)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        match row {
            Some(starpath_id) => self.get_starpath(starpath_id).await,
            None => Ok(None),
        }
    }

    pub async fn update_starpath_content_status(
        &self,
        starpath_id: Uuid,
        content_status: &str,
    ) -> Result<Option<Starpath>, AppError> {
        if !matches!(content_status, "active" | "archived") {
            return Err(AppError::BadRequest(
                "content_status must be active or archived".into(),
            ));
        }

        let row = sqlx::query_scalar::<_, Uuid>(
            r#"
            UPDATE starpaths
            SET content_status = $2
            WHERE starpath_id = $1
            RETURNING starpath_id
            "#,
        )
        .bind(starpath_id)
        .bind(content_status)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        match row {
            Some(starpath_id) => self.get_starpath(starpath_id).await,
            None => Ok(None),
        }
    }

    // =========================
    // DELETE /starpaths/:id
    // =========================
    pub async fn delete_starpath(&self, starpath_id: Uuid) -> Result<u64, AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM starpaths
            WHERE starpath_id = $1
            "#,
        )
        .bind(starpath_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(result.rows_affected())
    }

    // =========================
    // GET /starpaths/{id}/labs
    // =========================
    pub async fn get_starpath_labs(&self, starpath_id: Uuid) -> Result<Vec<StarpathLab>, AppError> {
        let rows = sqlx::query_as::<_, StarpathLabRow>(
            r#"
            SELECT starpath_id, lab_id, chapter_id, position
            FROM starpath_labs
            WHERE starpath_id = $1
            ORDER BY position ASC
            "#,
        )
        .bind(starpath_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(StarpathLab::from).collect())
    }

    pub async fn get_starpath_chapters(
        &self,
        starpath_id: Uuid,
    ) -> Result<Vec<StarpathChapter>, AppError> {
        let rows = sqlx::query_as::<_, StarpathChapterRow>(
            r#"
            SELECT chapter_id, starpath_id, name, position
            FROM starpath_chapters
            WHERE starpath_id = $1
            ORDER BY position ASC
            "#,
        )
        .bind(starpath_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(StarpathChapter::from).collect())
    }

    pub async fn create_starpath_chapter(
        &self,
        starpath_id: Uuid,
        input: CreateStarpathChapterInput,
    ) -> Result<StarpathChapter, AppError> {
        let name = input.name.trim();
        if name.is_empty() {
            return Err(AppError::BadRequest("Chapter name cannot be empty".into()));
        }
        if name.len() > 160 {
            return Err(AppError::BadRequest("Chapter name too long".into()));
        }

        let row = sqlx::query_as::<_, StarpathChapterRow>(
            r#"
            INSERT INTO starpath_chapters (starpath_id, name, position)
            VALUES ($1, $2, $3)
            RETURNING chapter_id, starpath_id, name, position
            "#,
        )
        .bind(starpath_id)
        .bind(name)
        .bind(input.position)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Conflict(e.to_string()))?;

        Ok(StarpathChapter::from(row))
    }

    pub async fn update_starpath_chapter(
        &self,
        starpath_id: Uuid,
        chapter_id: Uuid,
        input: UpdateStarpathChapterInput,
    ) -> Result<Option<StarpathChapter>, AppError> {
        let name = input
            .name
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        if let Some(ref name) = name {
            if name.len() > 160 {
                return Err(AppError::BadRequest("Chapter name too long".into()));
            }
        }

        let row = sqlx::query_as::<_, StarpathChapterRow>(
            r#"
            UPDATE starpath_chapters
            SET
                name = COALESCE($3, name),
                position = COALESCE($4, position)
            WHERE starpath_id = $1 AND chapter_id = $2
            RETURNING chapter_id, starpath_id, name, position
            "#,
        )
        .bind(starpath_id)
        .bind(chapter_id)
        .bind(name)
        .bind(input.position)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Conflict(e.to_string()))?;

        Ok(row.map(StarpathChapter::from))
    }

    pub async fn delete_starpath_chapter(
        &self,
        starpath_id: Uuid,
        chapter_id: Uuid,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM starpath_chapters
            WHERE starpath_id = $1 AND chapter_id = $2
            "#,
        )
        .bind(starpath_id)
        .bind(chapter_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Chapter not found in starpath".into()));
        }

        Ok(())
    }

    // ======================================
    // POST /starpaths/{id}/labs
    // ======================================
    pub async fn add_lab_to_starpath(
        &self,
        starpath_id: Uuid,
        input: AddStarpathLabInput,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO starpath_labs (starpath_id, lab_id, chapter_id, position)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(starpath_id)
        .bind(input.lab_id)
        .bind(input.chapter_id)
        .bind(input.position)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Conflict(e.to_string()))?;

        Ok(())
    }

    // =========================================
    // PUT /starpaths/{id}/labs/{lab_id}
    // =========================================
    pub async fn update_starpath_lab_position(
        &self,
        starpath_id: Uuid,
        lab_id: Uuid,
        position: Option<i32>,
        chapter_id: Option<Option<Uuid>>,
    ) -> Result<(), AppError> {
        let update_chapter = chapter_id.is_some();
        let chapter_id = chapter_id.flatten();

        let result = sqlx::query(
            r#"
            UPDATE starpath_labs
            SET
                position = COALESCE($3, position),
                chapter_id = CASE WHEN $4 THEN $5 ELSE chapter_id END
            WHERE starpath_id = $1 AND lab_id = $2
            "#,
        )
        .bind(starpath_id)
        .bind(lab_id)
        .bind(position)
        .bind(update_chapter)
        .bind(chapter_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Lab not found in starpath".into()));
        }

        Ok(())
    }

    // ==========================================
    // DELETE /starpaths/{id}/labs/{lab_id}
    // ==========================================
    pub async fn remove_lab_from_starpath(
        &self,
        starpath_id: Uuid,
        lab_id: Uuid,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM starpath_labs
            WHERE starpath_id = $1 AND lab_id = $2
            "#,
        )
        .bind(starpath_id)
        .bind(lab_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Lab not found in starpath".into()));
        }

        Ok(())
    }

    // =========================
    // POST /starpaths/:id/start
    // =========================
    pub async fn start_starpath(
        &self,
        user_id: Uuid,
        starpath_id: Uuid,
        is_admin: bool,
    ) -> Result<StarpathProgress, AppError> {
        self.ensure_starpath_can_start(user_id, starpath_id, is_admin)
            .await?;

        if let Some(row) = sqlx::query_as::<_, StarpathProgressRow>(
            r#"
            SELECT
                user_id,
                starpath_id,
                current_position,
                status,
                started_at,
                completed_at
            FROM user_starpath_progress
            WHERE user_id = $1 AND starpath_id = $2
            "#,
        )
        .bind(user_id)
        .bind(starpath_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        {
            return Ok(StarpathProgress::from(row));
        }

        let row = sqlx::query_as::<_, StarpathProgressRow>(
            r#"
            INSERT INTO user_starpath_progress (
                user_id,
                starpath_id,
                current_position,
                status
            )
            VALUES ($1, $2, 0, 'in_progress')
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(starpath_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(StarpathProgress::from(row))
    }

    // =================================
    // GET /starpaths/:id/progress
    // =================================
    pub async fn get_starpath_progress(
        &self,
        user_id: Uuid,
        starpath_id: Uuid,
    ) -> Result<Option<StarpathProgress>, AppError> {
        self.sync_starpath_progress(user_id, starpath_id).await?;

        let row = sqlx::query_as::<_, StarpathProgressRow>(
            r#"
            SELECT
                user_id,
                starpath_id,
                current_position,
                status,
                started_at,
                completed_at
            FROM user_starpath_progress
            WHERE user_id = $1
              AND starpath_id = $2
            "#,
        )
        .bind(user_id)
        .bind(starpath_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(row.map(StarpathProgress::from))
    }

    pub async fn get_starpath_analytics(
        &self,
        caller_user_id: Uuid,
        is_admin: bool,
        starpath_id: Uuid,
        window: &str,
    ) -> Result<StarpathAnalytics, AppError> {
        let starpath = self
            .get_starpath(starpath_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Starpath not found".into()))?;

        if !is_admin && starpath.creator_id != caller_user_id {
            return Err(AppError::Forbidden(
                "You are not allowed to inspect this starpath analytics".into(),
            ));
        }

        self.sync_all_starpath_progress(starpath_id).await?;

        let linked_labs = self.get_starpath_labs(starpath_id).await?;
        let progress_rows = self.list_starpath_progress_snapshots(starpath_id).await?;
        let (window_start, window_name) = normalize_window(window);

        let learners_started = progress_rows
            .iter()
            .filter(|row| row.started_at >= window_start)
            .count() as i64;
        let learners_completed = progress_rows
            .iter()
            .filter(|row| {
                row.completed_at
                    .is_some_and(|completed_at| completed_at >= window_start)
            })
            .count() as i64;

        let completion_rate = if learners_started <= 0 {
            0.0
        } else {
            (learners_completed as f64 / learners_started as f64) * 100.0
        };

        let linked_labs_count = linked_labs.len() as i32;
        let progress_distribution = build_progress_distribution(&progress_rows, linked_labs_count);
        let strongest_dropoff = detect_strongest_dropoff(&progress_distribution);

        Ok(StarpathAnalytics {
            window: window_name,
            starpath_id,
            linked_labs_count,
            learners_started,
            learners_completed,
            completion_rate,
            progress_distribution,
            strongest_dropoff,
            generated_at: Utc::now().to_rfc3339(),
        })
    }

    pub async fn ensure_starpath_access(
        &self,
        user_id: Uuid,
        starpath_id: Uuid,
        is_admin: bool,
    ) -> Result<Starpath, AppError> {
        let starpath = self
            .get_starpath(starpath_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Starpath not found".into()))?;

        if is_admin || starpath.creator_id == user_id {
            return Ok(starpath);
        }

        let public_active = starpath.visibility == StarpathVisibility::Public
            && starpath.content_status.eq_ignore_ascii_case("active");

        if public_active
            || self
                .user_has_group_access_to_starpath(user_id, starpath_id)
                .await?
        {
            return Ok(starpath);
        }

        Err(AppError::Forbidden(
            "You are not allowed to access this starpath".into(),
        ))
    }

    async fn ensure_starpath_can_start(
        &self,
        user_id: Uuid,
        starpath_id: Uuid,
        is_admin: bool,
    ) -> Result<(), AppError> {
        let starpath = self
            .get_starpath(starpath_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Starpath not found".into()))?;

        if starpath.content_status != "active" {
            return Err(AppError::Forbidden(
                "This starpath is archived and cannot be started".into(),
            ));
        }

        if is_admin
            || starpath.creator_id == user_id
            || starpath.visibility == StarpathVisibility::Public
        {
            return Ok(());
        }

        if self
            .user_has_group_access_to_starpath(user_id, starpath_id)
            .await?
        {
            return Ok(());
        }

        Err(AppError::Forbidden(
            "You are not allowed to start this private starpath".into(),
        ))
    }

    async fn user_has_group_access_to_starpath(
        &self,
        user_id: Uuid,
        starpath_id: Uuid,
    ) -> Result<bool, AppError> {
        let endpoint = self
            .groups_ms_base
            .join("/internal/access/starpath")
            .map_err(|_| AppError::Internal("Invalid Groups MS endpoint".into()))?;

        let body = cloud_run_auth::with_cloud_run_auth(
            self.http.get(endpoint.as_str()),
            endpoint.as_str(),
        )
        .await
        .query(&[
            ("user_id", user_id.to_string()),
            ("starpath_id", starpath_id.to_string()),
        ])
        .header(
            "x-altair-internal-token",
            std::env::var("INTERNAL_SERVICE_TOKEN")
                .unwrap_or_else(|_| "local-dev-token".to_string()),
        )
        .send()
        .await
        .map_err(|_| AppError::Internal("Groups MS unreachable".into()))?
        .json::<serde_json::Value>()
        .await
        .map_err(|_| AppError::Internal("Invalid Groups response".into()))?;

        Ok(body
            .get("data")
            .and_then(|value| value.as_bool())
            .unwrap_or(false))
    }

    pub async fn list_starpath_progress_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<StarpathProgress>, AppError> {
        let rows = sqlx::query_as::<_, StarpathProgressRow>(
            r#"
            SELECT
                user_id,
                starpath_id,
                current_position,
                status,
                started_at,
                completed_at
            FROM user_starpath_progress
            WHERE user_id = $1
            ORDER BY started_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(StarpathProgress::from).collect())
    }

    async fn sync_all_starpath_progress(&self, starpath_id: Uuid) -> Result<(), AppError> {
        let user_ids = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT user_id
            FROM user_starpath_progress
            WHERE starpath_id = $1
            "#,
        )
        .bind(starpath_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        for user_id in user_ids {
            self.sync_starpath_progress(user_id, starpath_id).await?;
        }

        Ok(())
    }

    async fn sync_starpath_progress(
        &self,
        user_id: Uuid,
        starpath_id: Uuid,
    ) -> Result<(), AppError> {
        let linked_labs = self.get_starpath_labs(starpath_id).await?;
        if linked_labs.is_empty() {
            return Ok(());
        }

        let existing_progress = sqlx::query_as::<_, StarpathProgressRow>(
            r#"
            SELECT
                user_id,
                starpath_id,
                current_position,
                status,
                started_at,
                completed_at
            FROM user_starpath_progress
            WHERE user_id = $1
              AND starpath_id = $2
            "#,
        )
        .bind(user_id)
        .bind(starpath_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let completed_sessions = self.fetch_completed_lab_sessions_for_user(user_id).await?;
        let completed_lab_times = completed_sessions
            .into_iter()
            .filter(|session| session.status.eq_ignore_ascii_case("completed"))
            .filter_map(|session| {
                session
                    .completed_at
                    .map(|completed_at| (session.lab_id, completed_at))
            })
            .collect::<std::collections::HashMap<_, _>>();

        if existing_progress.is_none() && completed_lab_times.is_empty() {
            return Ok(());
        }

        let next_position = compute_next_position(&linked_labs, &completed_lab_times);
        let linked_labs_count = linked_labs.len() as i32;
        let is_completed = linked_labs_count > 0 && next_position >= linked_labs_count;
        let was_completed = existing_progress
            .as_ref()
            .is_some_and(|row| row.status.eq_ignore_ascii_case("completed"));

        let started_at = existing_progress
            .as_ref()
            .map(|row| row.started_at)
            .unwrap_or_else(|| {
                completed_lab_times
                    .values()
                    .min()
                    .copied()
                    .unwrap_or_else(|| Utc::now().naive_utc())
            });

        let completed_at = if was_completed {
            existing_progress.as_ref().and_then(|row| row.completed_at)
        } else if is_completed {
            linked_labs
                .iter()
                .filter_map(|lab| completed_lab_times.get(&lab.lab_id).copied())
                .max()
        } else {
            None
        };

        let status = if was_completed || is_completed {
            "completed"
        } else {
            "in_progress"
        };

        sqlx::query(
            r#"
            INSERT INTO user_starpath_progress (
                user_id,
                starpath_id,
                current_position,
                status,
                started_at,
                completed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (user_id, starpath_id)
            DO UPDATE SET
                current_position = CASE
                    WHEN user_starpath_progress.status = 'completed'
                        THEN user_starpath_progress.current_position
                    ELSE EXCLUDED.current_position
                END,
                status = EXCLUDED.status,
                started_at = user_starpath_progress.started_at,
                completed_at = COALESCE(user_starpath_progress.completed_at, EXCLUDED.completed_at)
            "#,
        )
        .bind(user_id)
        .bind(starpath_id)
        .bind(next_position)
        .bind(status)
        .bind(started_at)
        .bind(completed_at)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(())
    }

    async fn fetch_completed_lab_sessions_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<SessionSummary>, AppError> {
        #[derive(Debug, Deserialize)]
        struct SessionApiResponse {
            data: Vec<SessionSummary>,
        }

        let endpoint = self
            .sessions_ms_base
            .join(&format!("/sessions/user/{user_id}"))
            .map_err(|_| AppError::Internal("Invalid Sessions MS endpoint".into()))?;

        let response = cloud_run_auth::with_cloud_run_auth(
            self.http.get(endpoint.as_str()),
            endpoint.as_str(),
        )
        .await
        .header("x-altair-user-id", user_id.to_string())
        .header("x-altair-roles", "learner")
        .header(
            "x-altair-gateway-token",
            std::env::var("GATEWAY_SHARED_TOKEN").unwrap_or_default(),
        )
        .send()
        .await
        .map_err(|_| AppError::Internal("Sessions MS unreachable".into()))?;

        if !response.status().is_success() {
            return Err(AppError::Internal(format!(
                "Sessions MS returned {} while syncing starpath progress",
                response.status()
            )));
        }

        let body = response
            .json::<SessionApiResponse>()
            .await
            .map_err(|_| AppError::Internal("Invalid Sessions MS response".into()))?;

        Ok(body.data)
    }

    async fn list_starpath_progress_snapshots(
        &self,
        starpath_id: Uuid,
    ) -> Result<Vec<StarpathProgressSnapshot>, AppError> {
        let rows = sqlx::query_as::<_, StarpathProgressRow>(
            r#"
            SELECT
                user_id,
                starpath_id,
                current_position,
                status,
                started_at,
                completed_at
            FROM user_starpath_progress
            WHERE starpath_id = $1
            "#,
        )
        .bind(starpath_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|row| StarpathProgressSnapshot {
                current_position: row.current_position,
                started_at: row.started_at,
                completed_at: row.completed_at,
            })
            .collect())
    }
}

fn compute_next_position(
    linked_labs: &[StarpathLab],
    completed_lab_times: &std::collections::HashMap<Uuid, NaiveDateTime>,
) -> i32 {
    let mut ordered = linked_labs.to_vec();
    ordered.sort_by_key(|lab| lab.position);

    let mut next_position = 0;
    for lab in ordered {
        if completed_lab_times.contains_key(&lab.lab_id) {
            next_position += 1;
        } else {
            break;
        }
    }

    next_position
}

fn normalize_window(window: &str) -> (NaiveDateTime, String) {
    match window {
        "30d" => ((Utc::now() - Duration::days(30)).naive_utc(), "30d".into()),
        _ => ((Utc::now() - Duration::days(7)).naive_utc(), "7d".into()),
    }
}

fn build_progress_distribution(
    rows: &[StarpathProgressSnapshot],
    linked_labs_count: i32,
) -> Vec<StarpathProgressPoint> {
    (0..=linked_labs_count)
        .map(|position| StarpathProgressPoint {
            position,
            reached_count: rows
                .iter()
                .filter(|row| row.current_position >= position)
                .count() as i64,
        })
        .collect()
}

fn detect_strongest_dropoff(points: &[StarpathProgressPoint]) -> Option<StarpathDropoff> {
    let mut best: Option<StarpathDropoff> = None;

    for window in points.windows(2) {
        let previous = &window[0];
        let next = &window[1];

        if previous.reached_count < 10 || previous.reached_count <= 0 {
            continue;
        }

        let drop_percent = ((previous.reached_count - next.reached_count) as f64
            / previous.reached_count as f64)
            * 100.0;

        if drop_percent < 40.0 {
            continue;
        }

        let candidate = StarpathDropoff {
            from_position: previous.position,
            to_position: next.position,
            previous_count: previous.reached_count,
            next_count: next.reached_count,
            drop_percent,
        };

        match &best {
            Some(current) if current.drop_percent >= candidate.drop_percent => {}
            _ => best = Some(candidate),
        }
    }

    best
}
