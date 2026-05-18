/**
 * @file starpath_progress — user progression tracking models.
 *
 * @remarks
 * Defines how a user's progress within a starpath is stored and exposed:
 *
 *  - Database model (`StarpathProgressRow`)
 *  - API model (`StarpathProgress`)
 *
 * Tracks progression state including position, status, and timestamps.
 *
 * Key characteristics:
 *
 *  - Maintains user progression within ordered starpaths
 *  - Supports lifecycle states via `status`
 *  - Includes start and completion timestamps
 *  - Simple conversion using `From`
 *
 * @packageDocumentation
 */
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnerStarpath {
    #[serde(flatten)]
    pub starpath: crate::models::starpath::Starpath,
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
