use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// =======================
/// DB representation
/// =======================

#[derive(Debug, Clone, FromRow)]
pub struct StarpathRow {
    pub starpath_id: Uuid,
    pub creator_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub difficulty: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

/// =======================
/// API representation
/// =======================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Starpath {
    pub starpath_id: Uuid,
    pub creator_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub difficulty: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

/// =======================
/// Conversion DB → API
/// =======================

impl From<StarpathRow> for Starpath {
    fn from(row: StarpathRow) -> Self {
        Self {
            starpath_id: row.starpath_id,
            creator_id: row.creator_id,
            name: row.name,
            description: row.description,
            difficulty: row.difficulty,
            created_at: row.created_at,
        }
    }
}
