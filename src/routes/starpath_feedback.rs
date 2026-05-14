use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        api::ApiResponse,
        starpath::StarpathVisibility,
        starpath_feedback::{
            CreateStarpathFeedbackRequest, EngagementWindowQuery, StarpathEngagementSummary,
            StarpathFeedback, StarpathFeedbackSummary, UpdateStarpathFeedbackReplyRequest,
            UpdateStarpathFeedbackRequest, UpdateStarpathFeedbackVoteRequest,
        },
    },
    services::extractor::{extract_caller, Caller},
    state::AppState,
};

fn is_admin(caller: &Caller) -> bool {
    caller.roles.iter().any(|role| role == "admin")
}

fn is_creator(caller: &Caller) -> bool {
    caller.roles.iter().any(|role| role == "creator")
}

fn is_learner(caller: &Caller) -> bool {
    caller.roles.iter().any(|role| role == "learner")
}

async fn ensure_starpath_feedback_access(
    state: &AppState,
    caller: &Caller,
    starpath_id: Uuid,
) -> Result<crate::models::starpath::Starpath, AppError> {
    state
        .starpaths_service
        .ensure_starpath_access(caller.user_id, starpath_id, is_admin(caller))
        .await
}

async fn ensure_learner_feedback_eligibility(
    state: &AppState,
    caller: &Caller,
    starpath_id: Uuid,
) -> Result<(), AppError> {
    if !is_learner(caller) {
        return Err(AppError::Forbidden(
            "Learner role is required to leave starpath feedback".into(),
        ));
    }

    let progress = state
        .starpaths_service
        .get_starpath_progress(caller.user_id, starpath_id)
        .await?;

    if matches!(
        progress.as_ref().map(|item| item.status.as_str()),
        Some("in_progress" | "finished" | "IN_PROGRESS" | "FINISHED")
    ) {
        return Ok(());
    }

    Err(AppError::Forbidden(
        "You must start the starpath before you can leave feedback".into(),
    ))
}

pub async fn get_starpath_feedbacks(
    State(state): State<AppState>,
    Path(starpath_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<StarpathFeedback>>>, AppError> {
    let caller = extract_caller(&headers)?;
    ensure_starpath_feedback_access(&state, &caller, starpath_id).await?;

    let feedbacks = state
        .starpath_feedback_service
        .get_feedbacks_by_starpath(starpath_id, caller.user_id)
        .await?;

    Ok(Json(ApiResponse::success(feedbacks)))
}

pub async fn get_starpath_engagement_summary(
    State(state): State<AppState>,
    Path(starpath_id): Path<Uuid>,
    headers: HeaderMap,
    Query(query): Query<EngagementWindowQuery>,
) -> Result<Json<ApiResponse<StarpathEngagementSummary>>, AppError> {
    let caller = extract_caller(&headers)?;
    ensure_starpath_feedback_access(&state, &caller, starpath_id).await?;

    let summary = state
        .starpath_feedback_service
        .get_engagement_summary(starpath_id, query.window.as_deref().unwrap_or("7d"))
        .await?;

    Ok(Json(ApiResponse::success(summary)))
}

pub async fn create_starpath_feedback(
    State(state): State<AppState>,
    Path(starpath_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<CreateStarpathFeedbackRequest>,
) -> Result<Json<ApiResponse<StarpathFeedback>>, AppError> {
    let caller = extract_caller(&headers)?;
    ensure_starpath_feedback_access(&state, &caller, starpath_id).await?;
    ensure_learner_feedback_eligibility(&state, &caller, starpath_id).await?;

    let feedback = state
        .starpath_feedback_service
        .create_feedback(
            starpath_id,
            caller.user_id,
            &caller.pseudo,
            &payload.content,
            payload.rating,
        )
        .await?;

    Ok(Json(ApiResponse::success(feedback)))
}

pub async fn update_starpath_feedback(
    State(state): State<AppState>,
    Path(feedback_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<UpdateStarpathFeedbackRequest>,
) -> Result<Json<ApiResponse<StarpathFeedback>>, AppError> {
    let caller = extract_caller(&headers)?;
    let feedback = state
        .starpath_feedback_service
        .get_feedback_by_id(feedback_id, caller.user_id)
        .await?;

    ensure_starpath_feedback_access(&state, &caller, feedback.starpath_id).await?;

    if feedback.user_id != caller.user_id {
        return Err(AppError::Forbidden(
            "You can only edit your own feedback".into(),
        ));
    }

    let updated = state
        .starpath_feedback_service
        .update_feedback(feedback_id, caller.user_id, &payload.content, payload.rating)
        .await?;

    Ok(Json(ApiResponse::success(updated)))
}

pub async fn delete_starpath_feedback(
    State(state): State<AppState>,
    Path(feedback_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let caller = extract_caller(&headers)?;
    let feedback = state
        .starpath_feedback_service
        .get_feedback_by_id(feedback_id, caller.user_id)
        .await?;

    ensure_starpath_feedback_access(&state, &caller, feedback.starpath_id).await?;

    if feedback.user_id != caller.user_id && !is_admin(&caller) {
        return Err(AppError::Forbidden(
            "You can only delete your own feedback".into(),
        ));
    }

    state
        .starpath_feedback_service
        .soft_delete_feedback(feedback_id)
        .await?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn set_starpath_feedback_vote(
    State(state): State<AppState>,
    Path(feedback_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<UpdateStarpathFeedbackVoteRequest>,
) -> Result<Json<ApiResponse<StarpathFeedback>>, AppError> {
    let caller = extract_caller(&headers)?;
    let feedback = state
        .starpath_feedback_service
        .get_feedback_by_id(feedback_id, caller.user_id)
        .await?;

    ensure_starpath_feedback_access(&state, &caller, feedback.starpath_id).await?;

    let updated = state
        .starpath_feedback_service
        .set_vote(feedback_id, caller.user_id, payload.vote)
        .await?;

    Ok(Json(ApiResponse::success(updated)))
}

pub async fn upsert_starpath_feedback_reply(
    State(state): State<AppState>,
    Path(feedback_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<UpdateStarpathFeedbackReplyRequest>,
) -> Result<Json<ApiResponse<StarpathFeedback>>, AppError> {
    let caller = extract_caller(&headers)?;
    let feedback = state
        .starpath_feedback_service
        .get_feedback_by_id(feedback_id, caller.user_id)
        .await?;

    let starpath = ensure_starpath_feedback_access(&state, &caller, feedback.starpath_id).await?;
    let is_owner_creator = is_creator(&caller) && caller.user_id == starpath.creator_id;

    if !is_owner_creator {
        return Err(AppError::Forbidden(
            "Only the starpath creator can reply to this feedback".into(),
        ));
    }

    let updated = state
        .starpath_feedback_service
        .upsert_creator_reply(
            feedback_id,
            feedback.starpath_id,
            caller.user_id,
            &caller.pseudo,
            &payload.content,
        )
        .await?;

    Ok(Json(ApiResponse::success(updated)))
}

pub async fn delete_starpath_feedback_reply(
    State(state): State<AppState>,
    Path(feedback_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<StarpathFeedback>>, AppError> {
    let caller = extract_caller(&headers)?;
    let feedback = state
        .starpath_feedback_service
        .get_feedback_by_id(feedback_id, caller.user_id)
        .await?;

    let starpath = ensure_starpath_feedback_access(&state, &caller, feedback.starpath_id).await?;
    let is_owner_creator = is_creator(&caller) && caller.user_id == starpath.creator_id;

    if !is_owner_creator {
        return Err(AppError::Forbidden(
            "Only the starpath creator can delete this reply".into(),
        ));
    }

    let updated = state
        .starpath_feedback_service
        .delete_creator_reply(feedback_id, caller.user_id)
        .await?;

    Ok(Json(ApiResponse::success(updated)))
}

pub async fn get_starpath_feedback_summary(
    State(state): State<AppState>,
    Path(starpath_id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<StarpathFeedbackSummary>>, AppError> {
    let caller = extract_caller(&headers)?;
    let starpath = ensure_starpath_feedback_access(&state, &caller, starpath_id).await?;

    if starpath.visibility != StarpathVisibility::Public
        && caller.user_id != starpath.creator_id
        && !is_admin(&caller)
    {
        return Err(AppError::Forbidden(
            "You are not allowed to access this starpath feedback summary".into(),
        ));
    }

    let summary = state
        .starpath_feedback_service
        .get_feedback_summary(starpath_id, caller.user_id)
        .await?;

    Ok(Json(ApiResponse::success(summary)))
}
