use serde::Deserialize;
use uuid::Uuid;

/// Payload pour POST /starpaths
/// MVP : création manuelle d’un starpath
#[derive(Debug, Deserialize)]
pub struct CreateStarpathInput {
    /// Temporaire tant que l’auth n’est pas branchée
    /// (sera remplacé par user_id extrait du JWT)
    pub creator_id: Uuid,

    pub name: String,
    pub description: Option<String>,
    pub difficulty: Option<String>,
}

/// Payload pour PUT /starpaths/:id
/// MVP : modification simple
#[derive(Debug, Deserialize)]
pub struct UpdateStarpathInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub difficulty: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddStarpathLabInput {
    pub lab_id: Uuid,
    pub position: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStarpathLabInput {
    pub position: i32,
}
