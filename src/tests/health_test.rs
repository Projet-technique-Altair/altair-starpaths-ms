use axum::{body::Body, response::IntoResponse};
use serde_json::Value;

use altair_starpaths_ms::routes::health::health;

#[tokio::test]
async fn health_returns_ok_json() {
    let resp = health().await.into_response();
    assert_eq!(resp.status(), 200);

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["status"], "ok");
}
