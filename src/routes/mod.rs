/**
 * @file routes — application route registration.
 *
 * @remarks
 * Defines and registers all HTTP routes for the Starpaths service.
 *
 *  - Mounts feature routes (starpaths, health)
 *  - Binds endpoints to their handlers
 *  - Attaches shared application state (`AppState`)
 *
 * Route categories:
 *
 *  - Health (`/health`)
 *  - Starpaths CRUD (`/starpaths`, `/mystarpaths`)
 *  - Search (`/search`)
 *  - Lab composition (add/update/remove labs)
 *  - Progress tracking (`/start`, `/progress`)
 *
 * Key characteristics:
 *
 *  - Centralized routing configuration
 *  - Clear separation between routing and handler logic
 *  - Supports both public and restricted endpoints
 *
 * @packageDocumentation
 */
use axum::{
    routing::{get, post, put},
    Router,
};

use crate::state::AppState;

use crate::routes::{
    health::health,
    starpaths::{
        add_starpath_lab, create_starpath, delete_starpath, delete_starpath_lab, get_starpath,
        get_starpath_labs, list_admin_user_starpath_progress, list_starpaths, list_starpaths_admin,
        my_starpaths, search_starpaths, update_starpath, update_starpath_content_status_admin,
        update_starpath_lab,
    },
};

pub mod health;
pub mod starpaths;

pub fn init_routes() -> Router<AppState> {
    Router::new()
        // Health
        .route("/health", get(health))
        // Starpaths CRUD
        .route("/admin/starpaths", get(list_starpaths_admin))
        .route(
            "/admin/starpaths/{id}/content-status",
            axum::routing::patch(update_starpath_content_status_admin),
        )
        .route(
            "/admin/users/{id}/progress",
            get(list_admin_user_starpath_progress),
        )
        .route("/starpaths", get(list_starpaths).post(create_starpath))
        .route("/mystarpaths", get(my_starpaths))
        .route("/search", get(search_starpaths))
        .route(
            "/starpaths/{id}",
            get(get_starpath)
                .put(update_starpath)
                .delete(delete_starpath),
        )
        // ⭐ Starpath Labs
        .route(
            "/starpaths/{id}/labs",
            get(get_starpath_labs).post(add_starpath_lab),
        )
        .route(
            "/starpaths/{id}/labs/{lab_id}",
            put(update_starpath_lab).delete(delete_starpath_lab),
        )
        .route("/starpaths/{id}/start", post(starpaths::start_starpath))
        .route(
            "/starpaths/{id}/progress",
            get(starpaths::get_starpath_progress),
        )
}
