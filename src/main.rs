use tower_http::cors::{Any, CorsLayer};

mod error;
mod models;
mod routes;
mod services;
mod state;

use crate::routes::init_routes;
use crate::state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let state = AppState::new().await;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = init_routes().with_state(state).layer(cors);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3005")
        .await
        .expect("Failed to bind port 3005");

    println!("Starpaths MS running on http://localhost:3005");

    axum::serve(listener, app).await.unwrap();
}
