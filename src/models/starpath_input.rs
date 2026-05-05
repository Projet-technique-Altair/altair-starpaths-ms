/**
 * @file starpath_input — request payload models for starpath operations.
 *
 * @remarks
 * Defines input structures used by Starpaths endpoints:
 *
 *  - Create (`CreateStarpathInput`)
 *  - Update (`UpdateStarpathInput`)
 *  - Lab assignment (`AddStarpathLabInput`, `UpdateStarpathLabInput`)
 *
 * Key characteristics:
 *
 *  - Supports partial updates via optional fields
 *  - Strong typing with UUIDs for lab references
 *  - Minimal validation layer (handled at service level)
 *
 * @packageDocumentation
 */
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateStarpathInput {
    pub name: String,
    pub description: Option<String>,
    pub difficulty: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStarpathInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub difficulty: Option<String>,
    pub visibility: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddStarpathLabInput {
    pub lab_id: Uuid,
    pub position: i32,
    pub chapter_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStarpathLabInput {
    pub position: Option<i32>,
    #[serde(default)]
    pub chapter_id: Option<Option<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateStarpathChapterInput {
    pub name: String,
    pub position: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStarpathChapterInput {
    pub name: Option<String>,
    pub position: Option<i32>,
}
