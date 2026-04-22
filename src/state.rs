/**
 * @file state — shared application state.
 *
 * @remarks
 * Defines the global state for the Starpaths service.
 *
 *  - Holds initialized services (`StarpathsService`)
 *  - Manages database connection pool (`PgPool`)
 *
 * Initialization:
 *
 *  - Reads `DATABASE_URL` from environment
 *  - Establishes PostgreSQL connection
 *  - Instantiates service layer
 *
 * Key characteristics:
 *
 *  - Shared via Axum `State`
 *  - Cloneable for concurrent requests
 *  - Centralized dependency management
 *
 * @packageDocumentation
 */

use crate::services::starpaths_service::StarpathsService;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub starpaths_service: StarpathsService,
}

impl AppState {
    pub async fn new() -> Self {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let db = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to database");

        Self {
            starpaths_service: StarpathsService::new(db),
        }
    }
}
