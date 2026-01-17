use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    state::AppState,
    error::AppError,
    models::{
        api::ApiResponse,
        starpath::Starpath,
        starpath_progress::StarpathProgress,
        starpath_input::{CreateStarpathInput, UpdateStarpathInput},
    },
};


// ======================================================
// GET /starpaths (public)
// ======================================================
pub async fn list_starpaths(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<Starpath>>>, AppError> {
    let starpaths = state
        .starpaths_service
        .list_starpaths()
        .await?;

    Ok(Json(ApiResponse::success(starpaths)))
}

// ======================================================
// GET /starpaths/:id (public)
// ======================================================
pub async fn get_starpath(
    State(state): State<AppState>,
    Path(starpath_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Starpath>>, AppError> {
    let starpath = state
        .starpaths_service
        .get_starpath(starpath_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Starpath not found".into()))?;

    Ok(Json(ApiResponse::success(starpath)))
}

// ======================================================
// POST /starpaths (public – MVP, pas d’auth)
// ======================================================
pub async fn create_starpath(
    State(state): State<AppState>,
    Json(input): Json<CreateStarpathInput>,
) -> Result<Json<ApiResponse<Starpath>>, AppError> {
    let starpath = state
        .starpaths_service
        .create_starpath(input)
        .await?;

    Ok(Json(ApiResponse::success(starpath)))
}

// ======================================================
// PUT /starpaths/:id (public – MVP)
// ======================================================
pub async fn update_starpath(
    State(state): State<AppState>,
    Path(starpath_id): Path<Uuid>,
    Json(input): Json<UpdateStarpathInput>,
) -> Result<Json<ApiResponse<Starpath>>, AppError> {
    let starpath = state
        .starpaths_service
        .update_starpath(starpath_id, input)
        .await?
        .ok_or_else(|| AppError::NotFound("Starpath not found".into()))?;

    Ok(Json(ApiResponse::success(starpath)))
}

// ======================================================
// DELETE /starpaths/:id (public – MVP)
// ======================================================
pub async fn delete_starpath(
    State(state): State<AppState>,
    Path(starpath_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let affected = state
        .starpaths_service
        .delete_starpath(starpath_id)
        .await?;

    if affected == 0 {
        return Err(AppError::NotFound("Starpath not found".into()));
    }

    Ok(Json(ApiResponse::success(())))
}


use crate::models::starpath_lab::StarpathLab;
use crate::models::starpath_input::{
    AddStarpathLabInput,
    UpdateStarpathLabInput,
};


pub async fn get_starpath_labs(
    State(state): State<AppState>,
    Path(starpath_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<StarpathLab>>>, AppError> {
    let labs = state
        .starpaths_service
        .get_starpath_labs(starpath_id)
        .await?;

    Ok(Json(ApiResponse::success(labs)))
}


pub async fn add_starpath_lab(
    State(state): State<AppState>,
    Path(starpath_id): Path<Uuid>,
    Json(input): Json<AddStarpathLabInput>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state
        .starpaths_service
        .add_lab_to_starpath(starpath_id, input)
        .await?;

    Ok(Json(ApiResponse::success(())))
}


pub async fn update_starpath_lab(
    State(state): State<AppState>,
    Path((starpath_id, lab_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateStarpathLabInput>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state
        .starpaths_service
        .update_starpath_lab_position(
            starpath_id,
            lab_id,
            input.position,
        )
        .await?;

    Ok(Json(ApiResponse::success(())))
}


pub async fn delete_starpath_lab(
    State(state): State<AppState>,
    Path((starpath_id, lab_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    state
        .starpaths_service
        .remove_lab_from_starpath(starpath_id, lab_id)
        .await?;

    Ok(Json(ApiResponse::success(())))
}




// ======================================================
// POST /starpaths/:id/start (MVP sans auth)
// ======================================================
pub async fn start_starpath(
    State(state): State<AppState>,
    Path(starpath_id): Path<Uuid>,
    Json(user_id): Json<Uuid>, // MVP TEMPORAIRE
) -> Result<Json<ApiResponse<StarpathProgress>>, AppError> {

    let progress = state
        .starpaths_service
        .start_starpath(user_id, starpath_id)
        .await?;

    Ok(Json(ApiResponse::success(progress)))
}



// ======================================================
// GET /starpaths/:id/progress (MVP sans auth)
// ======================================================
pub async fn get_starpath_progress(
    State(state): State<AppState>,
    Path(starpath_id): Path<Uuid>,
    Json(user_id): Json<Uuid>, // MVP TEMPORAIRE
) -> Result<Json<ApiResponse<StarpathProgress>>, AppError> {

    let progress = state
        .starpaths_service
        .get_starpath_progress(user_id, starpath_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Progress not found".into()))?;

    Ok(Json(ApiResponse::success(progress)))
}
