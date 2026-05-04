use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct StarpathChapterRow {
    #[allow(dead_code)]
    pub starpath_id: Uuid,
    pub chapter_id: Uuid,
    pub name: String,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarpathChapter {
    pub chapter_id: Uuid,
    pub name: String,
    pub position: i32,
}

impl From<StarpathChapterRow> for StarpathChapter {
    fn from(row: StarpathChapterRow) -> Self {
        Self {
            chapter_id: row.chapter_id,
            name: row.name,
            position: row.position,
        }
    }
}
