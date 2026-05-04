/**
 * @file starpath_lab — starpath lab association models.
 *
 * @remarks
 * Defines the relationship between starpaths and their labs:
 *
 *  - Database representation (`StarpathLabRow`)
 *  - API-facing model (`StarpathLab`)
 *
 * Each lab is linked to a starpath with a specific position,
 * defining its order within the learning path.
 *
 * Key characteristics:
 *
 *  - Separation between DB model and API model
 *  - Ordered structure via `position`
 *  - Simple conversion using `From`
 *
 * @packageDocumentation
 */
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct StarpathLabRow {
    #[allow(dead_code)]
    pub starpath_id: Uuid,
    pub lab_id: Uuid,
    pub chapter_id: Option<Uuid>,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarpathLab {
    pub lab_id: Uuid,
    pub chapter_id: Option<Uuid>,
    pub position: i32,
}

impl From<StarpathLabRow> for StarpathLab {
    fn from(row: StarpathLabRow) -> Self {
        Self {
            lab_id: row.lab_id,
            chapter_id: row.chapter_id,
            position: row.position,
        }
    }
}
