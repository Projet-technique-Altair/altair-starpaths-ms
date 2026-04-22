/**
 * @file extractor — caller identity extraction from request headers.
 *
 * @remarks
 * Extracts authenticated user information from headers injected by the gateway:
 *
 *  - `user_id` from `x-altair-user-id`
 *  - `roles` from `x-altair-roles` (comma-separated)
 *
 * Returns a `Caller` struct used for authorization logic in services.
 *
 * Key characteristics:
 *
 *  - Fails with `Unauthorized` if user identity is missing or invalid
 *  - Defaults to empty roles if not provided
 *  - Lightweight parsing (no external calls)
 *
 * @packageDocumentation
 */

use uuid::Uuid;
use axum::http::HeaderMap;
use crate::error::AppError;


#[derive(Debug)]
pub struct Caller {
    pub user_id: Uuid,
    pub roles: Vec<String>,
}

pub fn extract_caller(headers: &HeaderMap) -> Result<Caller, AppError> {
    let user_id = headers
        .get("x-altair-user-id")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::Unauthorized("Missing caller identity".to_string()))?;


    let roles = headers
        .get("x-altair-roles")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').map(|r| r.to_string()).collect())
        .unwrap_or_default();

    Ok(Caller { user_id, roles })
}
