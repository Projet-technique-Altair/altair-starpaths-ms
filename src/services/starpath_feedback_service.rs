use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::starpath_feedback::{
        StarpathEngagementSummary, StarpathFeedback, StarpathFeedbackRow, StarpathFeedbackSummary,
        StarpathFeedbackVoteValue,
    },
};

#[derive(Clone)]
pub struct StarpathFeedbackService {
    db: PgPool,
}

impl StarpathFeedbackService {
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    pub async fn ensure_schema(&self) -> Result<(), AppError> {
        // Starpath feedback schema is now owned by altair-infra/postgres/starpaths.
        Ok(())
    }

    pub async fn get_feedbacks_by_starpath(
        &self,
        starpath_id: Uuid,
        caller_id: Uuid,
    ) -> Result<Vec<StarpathFeedback>, AppError> {
        let rows = sqlx::query_as::<_, StarpathFeedbackRow>(
            r#"
            SELECT
                f.feedback_id,
                f.starpath_id,
                f.user_id,
                f.user_pseudo,
                f.content,
                f.rating,
                f.created_at,
                f.updated_at,
                f.deleted_at,
                f.creator_seen_at,
                f.creator_resolved_at,
                COALESCE(COUNT(*) FILTER (WHERE v.vote = 'like'), 0)::BIGINT AS likes_count,
                COALESCE(COUNT(*) FILTER (WHERE v.vote = 'dislike'), 0)::BIGINT AS dislikes_count,
                MAX(CASE WHEN v.user_id = $2 THEN v.vote END) AS current_user_vote,
                r.reply_id,
                r.feedback_id AS reply_feedback_id,
                r.starpath_id AS reply_starpath_id,
                r.creator_id AS reply_creator_id,
                r.creator_pseudo AS reply_creator_pseudo,
                r.content AS reply_content,
                r.created_at AS reply_created_at,
                r.updated_at AS reply_updated_at,
                r.deleted_at AS reply_deleted_at
            FROM starpath_feedbacks f
            LEFT JOIN starpath_feedback_votes v ON v.feedback_id = f.feedback_id
            LEFT JOIN starpath_feedback_replies r ON r.feedback_id = f.feedback_id AND r.deleted_at IS NULL
            WHERE f.starpath_id = $1
              AND f.deleted_at IS NULL
            GROUP BY
                f.feedback_id,
                f.starpath_id,
                f.user_id,
                f.user_pseudo,
                f.content,
                f.rating,
                f.created_at,
                f.updated_at,
                f.deleted_at,
                f.creator_seen_at,
                f.creator_resolved_at,
                r.reply_id,
                r.feedback_id,
                r.starpath_id,
                r.creator_id,
                r.creator_pseudo,
                r.content,
                r.created_at,
                r.updated_at,
                r.deleted_at
            ORDER BY f.created_at DESC
            "#,
        )
        .bind(starpath_id)
        .bind(caller_id)
        .fetch_all(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        rows.into_iter().map(StarpathFeedback::try_from).collect()
    }

    pub async fn create_feedback(
        &self,
        starpath_id: Uuid,
        user_id: Uuid,
        user_pseudo: &str,
        content: &str,
        rating: i32,
    ) -> Result<StarpathFeedback, AppError> {
        let trimmed = content.trim();
        let pseudo = user_pseudo.trim();

        if trimmed.is_empty() {
            return Err(AppError::BadRequest(
                "Feedback content cannot be empty".into(),
            ));
        }

        if trimmed.len() > 2_000 {
            return Err(AppError::BadRequest("Feedback content is too long".into()));
        }

        if pseudo.is_empty() {
            return Err(AppError::BadRequest(
                "Feedback author pseudo cannot be empty".into(),
            ));
        }

        if !(1..=5).contains(&rating) {
            return Err(AppError::BadRequest(
                "Starpath rating must be between 1 and 5".into(),
            ));
        }

        let feedback_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO starpath_feedbacks (
                feedback_id,
                starpath_id,
                user_id,
                user_pseudo,
                content,
                rating
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(feedback_id)
        .bind(starpath_id)
        .bind(user_id)
        .bind(pseudo)
        .bind(trimmed)
        .bind(rating)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.get_feedback_by_id(feedback_id, user_id).await
    }

    pub async fn update_feedback(
        &self,
        feedback_id: Uuid,
        caller_id: Uuid,
        content: &str,
        rating: Option<i32>,
    ) -> Result<StarpathFeedback, AppError> {
        let trimmed = content.trim();

        if trimmed.is_empty() {
            return Err(AppError::BadRequest(
                "Feedback content cannot be empty".into(),
            ));
        }

        if trimmed.len() > 2_000 {
            return Err(AppError::BadRequest("Feedback content is too long".into()));
        }

        if let Some(value) = rating {
            if !(1..=5).contains(&value) {
                return Err(AppError::BadRequest(
                    "Starpath rating must be between 1 and 5".into(),
                ));
            }
        }

        let result = sqlx::query(
            r#"
            UPDATE starpath_feedbacks
            SET
                content = $2,
                rating = COALESCE($4, rating),
                updated_at = NOW()
            WHERE feedback_id = $1
              AND user_id = $3
              AND deleted_at IS NULL
            "#,
        )
        .bind(feedback_id)
        .bind(trimmed)
        .bind(caller_id)
        .bind(rating)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::Forbidden(
                "You can only edit your own active feedbacks".into(),
            ));
        }

        self.get_feedback_by_id(feedback_id, caller_id).await
    }

    pub async fn soft_delete_feedback(&self, feedback_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE starpath_feedbacks
            SET
                deleted_at = NOW(),
                updated_at = NOW()
            WHERE feedback_id = $1
              AND deleted_at IS NULL
            "#,
        )
        .bind(feedback_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Feedback not found".into()));
        }

        sqlx::query(
            r#"
            DELETE FROM starpath_feedback_votes
            WHERE feedback_id = $1
            "#,
        )
        .bind(feedback_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        sqlx::query(
            r#"
            UPDATE starpath_feedback_replies
            SET
                deleted_at = NOW(),
                updated_at = NOW()
            WHERE feedback_id = $1
              AND deleted_at IS NULL
            "#,
        )
        .bind(feedback_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(())
    }

    pub async fn set_vote(
        &self,
        feedback_id: Uuid,
        user_id: Uuid,
        vote: Option<StarpathFeedbackVoteValue>,
    ) -> Result<StarpathFeedback, AppError> {
        if let Some(vote) = vote {
            let vote_value = match vote {
                StarpathFeedbackVoteValue::Like => "like",
                StarpathFeedbackVoteValue::Dislike => "dislike",
            };

            sqlx::query(
                r#"
                INSERT INTO starpath_feedback_votes (
                    feedback_id,
                    user_id,
                    vote
                )
                VALUES ($1, $2, $3)
                ON CONFLICT (feedback_id, user_id)
                DO UPDATE SET
                    vote = EXCLUDED.vote,
                    updated_at = NOW()
                "#,
            )
            .bind(feedback_id)
            .bind(user_id)
            .bind(vote_value)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        } else {
            sqlx::query(
                r#"
                DELETE FROM starpath_feedback_votes
                WHERE feedback_id = $1
                  AND user_id = $2
                "#,
            )
            .bind(feedback_id)
            .bind(user_id)
            .execute(&self.db)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        }

        self.get_feedback_by_id(feedback_id, user_id).await
    }

    pub async fn get_feedback_by_id(
        &self,
        feedback_id: Uuid,
        caller_id: Uuid,
    ) -> Result<StarpathFeedback, AppError> {
        let row = sqlx::query_as::<_, StarpathFeedbackRow>(
            r#"
            SELECT
                f.feedback_id,
                f.starpath_id,
                f.user_id,
                f.user_pseudo,
                f.content,
                f.rating,
                f.created_at,
                f.updated_at,
                f.deleted_at,
                f.creator_seen_at,
                f.creator_resolved_at,
                COALESCE(COUNT(*) FILTER (WHERE v.vote = 'like'), 0)::BIGINT AS likes_count,
                COALESCE(COUNT(*) FILTER (WHERE v.vote = 'dislike'), 0)::BIGINT AS dislikes_count,
                MAX(CASE WHEN v.user_id = $2 THEN v.vote END) AS current_user_vote,
                r.reply_id,
                r.feedback_id AS reply_feedback_id,
                r.starpath_id AS reply_starpath_id,
                r.creator_id AS reply_creator_id,
                r.creator_pseudo AS reply_creator_pseudo,
                r.content AS reply_content,
                r.created_at AS reply_created_at,
                r.updated_at AS reply_updated_at,
                r.deleted_at AS reply_deleted_at
            FROM starpath_feedbacks f
            LEFT JOIN starpath_feedback_votes v ON v.feedback_id = f.feedback_id
            LEFT JOIN starpath_feedback_replies r ON r.feedback_id = f.feedback_id AND r.deleted_at IS NULL
            WHERE f.feedback_id = $1
            GROUP BY
                f.feedback_id,
                f.starpath_id,
                f.user_id,
                f.user_pseudo,
                f.content,
                f.rating,
                f.created_at,
                f.updated_at,
                f.deleted_at,
                f.creator_seen_at,
                f.creator_resolved_at,
                r.reply_id,
                r.feedback_id,
                r.starpath_id,
                r.creator_id,
                r.creator_pseudo,
                r.content,
                r.created_at,
                r.updated_at,
                r.deleted_at
            "#,
        )
        .bind(feedback_id)
        .bind(caller_id)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Feedback not found".into()))?;

        StarpathFeedback::try_from(row)
    }

    pub async fn get_feedback_summary(
        &self,
        starpath_id: Uuid,
        caller_id: Uuid,
    ) -> Result<StarpathFeedbackSummary, AppError> {
        let row = sqlx::query_as::<_, (Option<f64>, i64, Option<i32>)>(
            r#"
            SELECT
                AVG(f.rating)::DOUBLE PRECISION AS average_rating,
                COUNT(f.feedback_id)::BIGINT AS rating_count,
                (
                    SELECT f2.rating
                    FROM starpath_feedbacks f2
                    WHERE f2.starpath_id = $1
                      AND f2.user_id = $2
                      AND f2.deleted_at IS NULL
                    ORDER BY f2.created_at DESC
                    LIMIT 1
                ) AS current_user_rating
            FROM starpath_feedbacks f
            WHERE f.starpath_id = $1
              AND f.deleted_at IS NULL
            "#,
        )
        .bind(starpath_id)
        .bind(caller_id)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(StarpathFeedbackSummary {
            average_rating: row.0.unwrap_or(0.0),
            rating_count: row.1,
            current_user_rating: row.2,
        })
    }

    pub async fn upsert_creator_reply(
        &self,
        feedback_id: Uuid,
        starpath_id: Uuid,
        creator_id: Uuid,
        creator_pseudo: &str,
        content: &str,
    ) -> Result<StarpathFeedback, AppError> {
        let trimmed = content.trim();
        let pseudo = creator_pseudo.trim();

        if trimmed.is_empty() {
            return Err(AppError::BadRequest("Reply content cannot be empty".into()));
        }

        if trimmed.len() > 2_000 {
            return Err(AppError::BadRequest("Reply content is too long".into()));
        }

        if pseudo.is_empty() {
            return Err(AppError::BadRequest("Creator pseudo cannot be empty".into()));
        }

        sqlx::query(
            r#"
            INSERT INTO starpath_feedback_replies (
                reply_id,
                feedback_id,
                starpath_id,
                creator_id,
                creator_pseudo,
                content
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (feedback_id)
            DO UPDATE SET
                creator_id = EXCLUDED.creator_id,
                creator_pseudo = EXCLUDED.creator_pseudo,
                content = EXCLUDED.content,
                updated_at = NOW(),
                deleted_at = NULL
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(feedback_id)
        .bind(starpath_id)
        .bind(creator_id)
        .bind(pseudo)
        .bind(trimmed)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        self.get_feedback_by_id(feedback_id, creator_id).await
    }

    pub async fn delete_creator_reply(
        &self,
        feedback_id: Uuid,
        caller_id: Uuid,
    ) -> Result<StarpathFeedback, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE starpath_feedback_replies
            SET
                deleted_at = NOW(),
                updated_at = NOW()
            WHERE feedback_id = $1
              AND creator_id = $2
              AND deleted_at IS NULL
            "#,
        )
        .bind(feedback_id)
        .bind(caller_id)
        .execute(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Creator reply not found".into()));
        }

        self.get_feedback_by_id(feedback_id, caller_id).await
    }

    pub async fn get_engagement_summary(
        &self,
        starpath_id: Uuid,
        window: &str,
    ) -> Result<StarpathEngagementSummary, AppError> {
        let cutoff = match window {
            "30d" => Utc::now() - Duration::days(30),
            _ => Utc::now() - Duration::days(7),
        };

        let comments_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM starpath_feedbacks
            WHERE starpath_id = $1
              AND deleted_at IS NULL
              AND created_at >= $2
            "#,
        )
        .bind(starpath_id)
        .bind(cutoff)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let negative_ratings_count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)::BIGINT
            FROM starpath_feedbacks
            WHERE starpath_id = $1
              AND deleted_at IS NULL
              AND created_at >= $2
              AND rating <= 2
            "#,
        )
        .bind(starpath_id)
        .bind(cutoff)
        .fetch_one(&self.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

        let (likes_count, dislikes_count, latest_vote_at) =
            sqlx::query_as::<_, (i64, i64, Option<chrono::DateTime<Utc>>)>(
                r#"
                SELECT
                    COALESCE(COUNT(*) FILTER (WHERE v.vote = 'like'), 0)::BIGINT AS likes_count,
                    COALESCE(COUNT(*) FILTER (WHERE v.vote = 'dislike'), 0)::BIGINT AS dislikes_count,
                    MAX(v.updated_at) AS latest_vote_at
                FROM starpath_feedback_votes v
                INNER JOIN starpath_feedbacks f ON f.feedback_id = v.feedback_id
                WHERE f.starpath_id = $1
                  AND f.deleted_at IS NULL
                  AND v.updated_at >= $2
                "#,
            )
            .bind(starpath_id)
            .bind(cutoff)
            .fetch_one(&self.db)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(StarpathEngagementSummary {
            window: match window {
                "30d" => "30d".into(),
                _ => "7d".into(),
            },
            comments_count,
            negative_ratings_count,
            likes_count,
            dislikes_count,
            latest_vote_at,
        })
    }
}
