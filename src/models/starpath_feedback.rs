use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StarpathFeedbackVoteValue {
    Like,
    Dislike,
}

#[derive(Debug, Clone, FromRow)]
pub struct StarpathFeedbackReplyRow {
    pub reply_id: Option<Uuid>,
    pub reply_feedback_id: Option<Uuid>,
    pub reply_starpath_id: Option<Uuid>,
    pub reply_creator_id: Option<Uuid>,
    pub reply_creator_pseudo: Option<String>,
    pub reply_content: Option<String>,
    pub reply_created_at: Option<DateTime<Utc>>,
    pub reply_updated_at: Option<DateTime<Utc>>,
    pub reply_deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarpathFeedbackReply {
    pub reply_id: Uuid,
    pub feedback_id: Uuid,
    pub starpath_id: Uuid,
    pub creator_id: Uuid,
    pub creator_pseudo: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl StarpathFeedbackReplyRow {
    pub fn into_reply(self) -> Result<Option<StarpathFeedbackReply>, AppError> {
        match (
            self.reply_id,
            self.reply_feedback_id,
            self.reply_starpath_id,
            self.reply_creator_id,
            self.reply_creator_pseudo,
            self.reply_content,
            self.reply_created_at,
            self.reply_updated_at,
        ) {
            (None, None, None, None, None, None, None, None) => Ok(None),
            (
                Some(reply_id),
                Some(feedback_id),
                Some(starpath_id),
                Some(creator_id),
                Some(creator_pseudo),
                Some(content),
                Some(created_at),
                Some(updated_at),
            ) => Ok(Some(StarpathFeedbackReply {
                reply_id,
                feedback_id,
                starpath_id,
                creator_id,
                creator_pseudo,
                content,
                created_at,
                updated_at,
                deleted_at: self.reply_deleted_at,
            })),
            _ => Err(AppError::Internal(
                "Incomplete starpath feedback reply row returned from database".into(),
            )),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct StarpathFeedbackRow {
    pub feedback_id: Uuid,
    pub starpath_id: Uuid,
    pub user_id: Uuid,
    pub user_pseudo: String,
    pub content: String,
    pub rating: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub creator_seen_at: Option<DateTime<Utc>>,
    pub creator_resolved_at: Option<DateTime<Utc>>,
    pub likes_count: i64,
    pub dislikes_count: i64,
    pub current_user_vote: Option<String>,
    pub reply_id: Option<Uuid>,
    pub reply_feedback_id: Option<Uuid>,
    pub reply_starpath_id: Option<Uuid>,
    pub reply_creator_id: Option<Uuid>,
    pub reply_creator_pseudo: Option<String>,
    pub reply_content: Option<String>,
    pub reply_created_at: Option<DateTime<Utc>>,
    pub reply_updated_at: Option<DateTime<Utc>>,
    pub reply_deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarpathFeedback {
    pub feedback_id: Uuid,
    pub starpath_id: Uuid,
    pub user_id: Uuid,
    pub user_pseudo: String,
    pub content: String,
    pub rating: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub creator_seen_at: Option<DateTime<Utc>>,
    pub creator_resolved_at: Option<DateTime<Utc>>,
    pub likes_count: i64,
    pub dislikes_count: i64,
    pub current_user_vote: Option<StarpathFeedbackVoteValue>,
    pub creator_reply: Option<StarpathFeedbackReply>,
}

impl TryFrom<StarpathFeedbackRow> for StarpathFeedback {
    type Error = AppError;

    fn try_from(row: StarpathFeedbackRow) -> Result<Self, Self::Error> {
        let current_user_vote = match row.current_user_vote.as_deref() {
            Some("like") => Some(StarpathFeedbackVoteValue::Like),
            Some("dislike") => Some(StarpathFeedbackVoteValue::Dislike),
            None => None,
            Some(other) => {
                return Err(AppError::Internal(format!(
                    "Invalid feedback vote in DB: {other}"
                )))
            }
        };

        Ok(Self {
            feedback_id: row.feedback_id,
            starpath_id: row.starpath_id,
            user_id: row.user_id,
            user_pseudo: row.user_pseudo,
            content: row.content,
            rating: row.rating,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            creator_seen_at: row.creator_seen_at,
            creator_resolved_at: row.creator_resolved_at,
            likes_count: row.likes_count,
            dislikes_count: row.dislikes_count,
            current_user_vote,
            creator_reply: StarpathFeedbackReplyRow {
                reply_id: row.reply_id,
                reply_feedback_id: row.reply_feedback_id,
                reply_starpath_id: row.reply_starpath_id,
                reply_creator_id: row.reply_creator_id,
                reply_creator_pseudo: row.reply_creator_pseudo,
                reply_content: row.reply_content,
                reply_created_at: row.reply_created_at,
                reply_updated_at: row.reply_updated_at,
                reply_deleted_at: row.reply_deleted_at,
            }
            .into_reply()?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateStarpathFeedbackRequest {
    pub content: String,
    pub rating: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStarpathFeedbackRequest {
    pub content: String,
    pub rating: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStarpathFeedbackVoteRequest {
    pub vote: Option<StarpathFeedbackVoteValue>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStarpathFeedbackReplyRequest {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarpathFeedbackSummary {
    pub average_rating: f64,
    pub rating_count: i64,
    pub current_user_rating: Option<i32>,
}
