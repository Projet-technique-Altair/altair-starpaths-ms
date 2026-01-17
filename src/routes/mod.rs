use axum::{
    routing::{get, post, put, delete},
    Router,
};

use crate::state::AppState;

use crate::routes::{
    health::health,
    starpaths::{
        list_starpaths,
        get_starpath,
        create_starpath,
        update_starpath,
        delete_starpath,
        get_starpath_labs,
        add_starpath_lab,
        update_starpath_lab,
        delete_starpath_lab,
    },
};

pub mod health;
pub mod starpaths;

pub fn init_routes() -> Router<AppState> {
    Router::new()
        // Health
        .route("/health", get(health))

        // Starpaths CRUD
        .route("/starpaths", get(list_starpaths).post(create_starpath))
        .route("/starpaths/:id", get(get_starpath).put(update_starpath).delete(delete_starpath),
        )

        // ⭐ Starpath Labs
        .route("/starpaths/:id/labs", get(get_starpath_labs).post(add_starpath_lab))
        .route("/starpaths/:id/labs/:lab_id", put(update_starpath_lab).delete(delete_starpath_lab),
        )

        .route("/starpaths/:id/start", post(starpaths::start_starpath))
        .route("/starpaths/:id/progress", get(starpaths::get_starpath_progress),
        )

}
