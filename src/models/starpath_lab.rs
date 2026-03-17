use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct StarpathLabRow {
    #[allow(dead_code)]
    pub starpath_id: Uuid,
    pub lab_id: Uuid,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarpathLab {
    pub lab_id: Uuid,
    pub position: i32,
}

impl From<StarpathLabRow> for StarpathLab {
    fn from(row: StarpathLabRow) -> Self {
        Self {
            lab_id: row.lab_id,
            position: row.position,
        }
    }
}
