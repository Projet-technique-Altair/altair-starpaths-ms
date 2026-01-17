use serde::{Serialize, Deserialize};
use uuid::Uuid;
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct StarpathLabRow {
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
