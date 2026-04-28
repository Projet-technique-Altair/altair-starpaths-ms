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
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::starpath::{Starpath, StarpathRow},
    models::starpath_input::{AddStarpathLabInput, UpdateStarpathInput},
    models::starpath_lab::{StarpathLab, StarpathLabRow},
    models::starpath_progress::{StarpathProgress, StarpathProgressRow},
};

#[derive(Clone)]
pub struct StarpathsService {
    db: PgPool,
}

impl StarpathsService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    // =========================
    // GET /starpaths
    // =========================
    pub async fn list_starpaths(&self) -> Result<Vec<Starpath>, AppError> {
        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
            starpath_id,
            creator_id,
            name,
            description,
            difficulty,
            visibility,
            content_status,
            created_at
            FROM starpaths
            WHERE visibility = 'public'
              AND content_status = 'active'
            ORDER BY created_at DESC
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
                starpath_id,
                creator_id,
                name,
                description,
                difficulty,
                visibility,
                content_status,
                created_at
            FROM starpaths
            WHERE ($1::TEXT IS NULL OR visibility = $1)
              AND ($2::TEXT IS NULL OR name ILIKE $2 OR description ILIKE $2)
              AND ($3::TEXT IS NULL OR content_status = $3)
            ORDER BY created_at DESC
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
            starpath_id,
            creator_id,
            name,
            description,
            difficulty,
            visibility,
            content_status,
            created_at
            FROM starpaths
            WHERE starpath_id = $1
            "#,
        )
        .bind(starpath_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(row.map(Starpath::try_from).transpose()?)
    }

    // ==========================
    // GET /mystarpaths (creator's starpaths only)
    // ==========================
    pub async fn my_starpaths(&self, creator_id: Uuid) -> Result<Vec<Starpath>, AppError> {
        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
                starpath_id,
                creator_id,
                name,
                description,
                difficulty,
                visibility,
                content_status,
                created_at
            FROM starpaths
            WHERE creator_id = $1
              AND content_status = 'active'
            ORDER BY created_at DESC
            "#,
        )
        .bind(creator_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        rows.into_iter().map(Starpath::try_from).collect()
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
                starpath_id,
                creator_id,
                name,
                description,
                difficulty,
                visibility,
                content_status,
                created_at
            FROM starpaths
            WHERE 
                name ILIKE $1
                AND content_status = 'active'
                AND (
                    visibility = 'public'
                    OR creator_id = $2
                )
            ORDER BY name
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

    // ==========================
    // GET /mystarpaths (creator's starpaths only)
    // ==========================
    pub async fn my_starpaths(&self, creator_id: Uuid) -> Result<Vec<Starpath>, AppError> {

        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
                starpath_id,
                creator_id,
                name,
                description,
                difficulty,
                created_at
            FROM starpaths
            WHERE creator_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(creator_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(Starpath::from).collect())
    }

    // ==========================
    // SEARCH STARPATHS
    // ==========================
    pub async fn search_starpaths(
        &self,
        query: String,
    ) -> Result<Vec<Starpath>, AppError> {

        let pattern = format!("%{}%", query);

        let rows = sqlx::query_as::<_, StarpathRow>(
            r#"
            SELECT
                starpath_id,
                creator_id,
                name,
                description,
                difficulty,
                created_at
            FROM starpaths
            WHERE name ILIKE $1
            ORDER BY name
            LIMIT 10
            "#,
        )
        .bind(pattern)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
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

        let row = sqlx::query_as::<_, StarpathRow>(
            r#"
            INSERT INTO starpaths (
                creator_id,
                name,
                description,
                difficulty,
                visibility
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(creator_id)
        .bind(name)
        .bind(description)
        .bind(difficulty)
        .bind(visibility)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Starpath::try_from(row)
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

        let row = sqlx::query_as::<_, StarpathRow>(
            r#"
            UPDATE starpaths
            SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                difficulty = COALESCE($4, difficulty),
                visibility = COALESCE($5, visibility)
            WHERE starpath_id = $1
            RETURNING *
            "#,
        )
        .bind(starpath_id)
        .bind(input.name)
        .bind(input.description)
        .bind(input.difficulty)
        .bind(visibility)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(row.map(Starpath::try_from).transpose()?)
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

        let row = sqlx::query_as::<_, StarpathRow>(
            r#"
            UPDATE starpaths
            SET content_status = $2
            WHERE starpath_id = $1
            RETURNING *
            "#,
        )
        .bind(starpath_id)
        .bind(content_status)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(row.map(Starpath::try_from).transpose()?)
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
            SELECT starpath_id, lab_id, position
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
            INSERT INTO starpath_labs (starpath_id, lab_id, position)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(starpath_id)
        .bind(input.lab_id)
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
        position: i32,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE starpath_labs
            SET position = $3
            WHERE starpath_id = $1 AND lab_id = $2
            "#,
        )
        .bind(starpath_id)
        .bind(lab_id)
        .bind(position)
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
    ) -> Result<StarpathProgress, AppError> {
        if let Some(row) = sqlx::query_as::<_, StarpathProgressRow>(
            r#"
            SELECT *
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
}
