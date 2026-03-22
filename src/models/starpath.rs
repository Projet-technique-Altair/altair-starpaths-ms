use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StarpathVisibility {
    Private,
    Public,
}


#[derive(Debug, Clone, FromRow)]
pub struct StarpathRow {
    pub starpath_id: Uuid,
    pub creator_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub difficulty: Option<String>,
    pub visibility: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Starpath {
    pub starpath_id: Uuid,
    pub creator_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub difficulty: Option<String>,
    pub visibility: StarpathVisibility,
    pub created_at: chrono::NaiveDateTime,
}

impl TryFrom<StarpathRow> for Starpath  {
    type Error = AppError;
    fn try_from(row: StarpathRow) -> Result<Self, Self::Error> {

        let visibility = match row.visibility.as_str() {
            "private" => StarpathVisibility::Private,
            "public" => StarpathVisibility::Public,
            other => {
                return Err(AppError::Internal(format!(
                    "Invalid starpath visibility in DB: {other}"
                )))
            }
        };

        Ok(Starpath {
            starpath_id: row.starpath_id,
            creator_id: row.creator_id,
            name: row.name,
            description: row.description,
            difficulty: row.difficulty,
            visibility,
            created_at: row.created_at,
        })
    }
}
