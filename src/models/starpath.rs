/**
 * @file starpath — core starpath domain model.
 *
 * @remarks
 * Defines the main Starpath entity and its visibility rules:
 *
 *  - Database model (`StarpathRow`)
 *  - API model (`Starpath`)
 *  - Visibility enum (`StarpathVisibility`)
 *
 * Handles conversion from raw database data to strongly-typed domain model.
 *
 * Key characteristics:
 *
 *  - Strong typing for visibility via enum (Public / Private)
 *  - Validation during conversion (`TryFrom`)
 *  - Prevents invalid visibility values from propagating
 *  - Includes metadata (creator, timestamps, difficulty)
 *
 * @packageDocumentation
 */
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
    pub content_status: String,
    pub rating_average: Option<f64>,
    pub rating_count: i64,
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
    pub content_status: String,
    pub rating_average: f64,
    pub rating_count: i64,
    pub created_at: chrono::NaiveDateTime,
}

impl TryFrom<StarpathRow> for Starpath {
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
            content_status: row.content_status,
            rating_average: row.rating_average.unwrap_or(0.0),
            rating_count: row.rating_count,
            created_at: row.created_at,
        })
    }
}
