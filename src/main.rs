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
use axum::http::HeaderValue;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

mod error;
mod models;
mod routes;
mod services;
mod state;

use crate::routes::init_routes;
use crate::state::AppState;

const DEFAULT_ALLOWED_ORIGINS: &str = "http://localhost:5173,http://localhost:3000";

fn parse_allowed_origins() -> Vec<HeaderValue> {
    std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| DEFAULT_ALLOWED_ORIGINS.to_string())
        .split(',')
        .filter_map(|origin| HeaderValue::from_str(origin.trim()).ok())
        .collect()
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let state = AppState::new().await;

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(parse_allowed_origins()))
        .allow_methods(Any)
        .allow_headers(Any);

    let app = init_routes().with_state(state).layer(cors);

    let port = std::env::var("PORT").unwrap_or("3005".to_string());

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap_or_else(|_| panic!("Failed to bind port {}", port));

    println!("Starpaths Ms running on http://localhost:{}", port);

    axum::serve(listener, app).await.unwrap();
}
