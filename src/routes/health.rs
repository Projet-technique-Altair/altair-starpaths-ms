/*use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::models::api::{ApiMeta, ApiResponse};

pub async fn health() -> impl IntoResponse {
    let meta = ApiMeta {
        request_id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let response = ApiResponse {
        success: true,
        data: None::<()>,
        meta,
    };

    (StatusCode::OK, Json(response))
}*/

use crate::state::AppState;
use axum::{extract::State, Json};
use serde_json::json;

pub async fn health(State(_): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "starpath ok",
        //"service": "labs-ms"
    }))
}
