/**
 * @file main — application entry point.
 *
 * @remarks
 * Bootstraps and starts the Starpaths microservice.
 *
 *  - Loads environment variables (`dotenv`)
 *  - Initializes application state (`AppState`)
 *  - Configures CORS policy
 *  - Registers routes and middleware
 *  - Starts the HTTP server (Axum)
 *
 * Key characteristics:
 *
 *  - Async runtime powered by Tokio
 *  - Configurable port via environment variable (`PORT`)
 *  - Global CORS enabled (to be restricted in production)
 *  - Centralized startup logic
 *
 * @packageDocumentation
 */
use axum::http::{HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod error;
mod models;
mod routes;
mod services;
mod state;

use crate::routes::init_routes;
use crate::state::AppState;

const DEFAULT_ALLOWED_ORIGINS: &str = "http://localhost:5173,http://localhost:3000";
const DEFAULT_ALLOWED_METHODS: &str = "GET,POST,PUT,PATCH,DELETE,OPTIONS";
const DEFAULT_ALLOWED_HEADERS: &str =
    "authorization,content-type,x-altair-user-id,x-altair-roles,x-altair-pseudo";

fn parse_allowed_origins() -> Vec<HeaderValue> {
    std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| DEFAULT_ALLOWED_ORIGINS.to_string())
        .split(',')
        .filter_map(|origin| HeaderValue::from_str(origin.trim()).ok())
        .collect()
}

fn parse_allowed_methods() -> Vec<Method> {
    std::env::var("ALLOWED_METHODS")
        .unwrap_or_else(|_| DEFAULT_ALLOWED_METHODS.to_string())
        .split(',')
        .filter_map(|method| Method::from_bytes(method.trim().as_bytes()).ok())
        .collect()
}

fn parse_allowed_headers() -> Vec<HeaderName> {
    std::env::var("ALLOWED_HEADERS")
        .unwrap_or_else(|_| DEFAULT_ALLOWED_HEADERS.to_string())
        .split(',')
        .filter_map(|header| {
            HeaderName::from_bytes(header.trim().to_ascii_lowercase().as_bytes()).ok()
        })
        .collect()
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let state = AppState::new().await;

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(parse_allowed_origins()))
        .allow_methods(parse_allowed_methods())
        .allow_headers(parse_allowed_headers());

    let app = init_routes().with_state(state).layer(cors);

    let port = std::env::var("PORT").unwrap_or("3005".to_string());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap_or_else(|_| panic!("Failed to bind port {}", port));

    info!(port = %port, "starpaths-ms started");

    axum::serve(listener, app).await.unwrap();
}
