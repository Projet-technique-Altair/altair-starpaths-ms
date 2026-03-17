use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct StarpathProgressRow {
    pub user_id: Uuid,
    pub starpath_id: Uuid,
    pub current_position: i32,
    pub status: String,
    pub started_at: chrono::NaiveDateTime,
    pub completed_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarpathProgress {
    pub user_id: Uuid,
    pub starpath_id: Uuid,
    pub current_position: i32,
    pub status: String,
    pub started_at: chrono::NaiveDateTime,
    pub completed_at: Option<chrono::NaiveDateTime>,
}

impl From<StarpathProgressRow> for StarpathProgress {
    fn from(row: StarpathProgressRow) -> Self {
        Self {
            user_id: row.user_id,
            starpath_id: row.starpath_id,
            current_position: row.current_position,
            status: row.status,
            started_at: row.started_at,
            completed_at: row.completed_at,
        }
    }
}
