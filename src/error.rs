/**
 * @file error — application-wide error handling.
 *
 * @remarks
 * Defines the unified error system used across the Groups service.
 *
 *  - Central `AppError` enum for all error types
 *  - Automatic conversion to HTTP responses via `IntoResponse`
 *  - Standardized error format (`ApiErrorResponse`)
 *
 * Error categories:
 *
 *  - NotFound, BadRequest, Unauthorized, Forbidden, Conflict, Internal
 *
 * Key characteristics:
 *
 *  - Maps errors to appropriate HTTP status codes
 *  - Provides consistent error structure (code, message, meta)
 *  - Generates request metadata for tracing/debugging
 *  - Converts database errors (`sqlx::Error`) into internal errors
 *
 * @packageDocumentation
 */
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};

use crate::models::api::{ApiError, ApiErrorResponse, ApiMeta};

#[derive(thiserror::Error, Debug)]
#[allow(dead_code)] // Certaines variantes seront utilisées par les routes
pub enum AppError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "RESOURCE_NOT_FOUND", msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg),
        };

        let error = ApiError {
            code: error_code.to_string(),
            message,
            details: None,
        };

        let meta = ApiMeta {
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let body = ApiErrorResponse {
            success: false,
            error,
            meta,
        };

        (status, Json(body)).into_response()
    }
}
