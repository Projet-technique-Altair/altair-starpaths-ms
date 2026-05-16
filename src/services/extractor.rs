use crate::error::AppError;
use axum::http::HeaderMap;
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

#[derive(Debug)]
pub struct Caller {
    pub user_id: Uuid,
    pub pseudo: String,
    pub roles: Vec<String>,
}

pub fn extract_caller(headers: &HeaderMap) -> Result<Caller, AppError> {
    ensure_gateway_origin(headers)?;
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

    let pseudo = headers
        .get("x-altair-pseudo")
        .and_then(|h| h.to_str().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(Caller {
        user_id,
        pseudo,
        roles,
    })
}

fn ensure_gateway_origin(headers: &HeaderMap) -> Result<(), AppError> {
    let expected = std::env::var("GATEWAY_SHARED_TOKEN")
        .map_err(|_| AppError::Unauthorized("Gateway shared token is not configured".into()))?;
    let provided = headers
        .get("x-altair-gateway-token")
        .and_then(|value| value.to_str().ok());
    if provided == Some(expected.as_str()) {
        Ok(())
    } else {
        Err(AppError::Unauthorized("Untrusted gateway origin".into()))
    }
}
