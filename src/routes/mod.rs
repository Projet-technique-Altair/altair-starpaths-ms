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
    starpath_feedback::{
        create_starpath_feedback, delete_starpath_feedback, delete_starpath_feedback_reply,
        get_starpath_engagement_summary, get_starpath_feedback_summary, get_starpath_feedbacks,
        set_starpath_feedback_vote, update_starpath_feedback, upsert_starpath_feedback_reply,
    },
    starpaths::{
        add_starpath_lab, create_starpath, create_starpath_chapter, delete_starpath,
        delete_starpath_chapter, delete_starpath_lab, get_starpath, get_starpath_analytics,
        get_starpath_chapters, get_starpath_labs, list_admin_user_starpath_progress,
        list_starpaths, list_starpaths_admin, my_starpaths, search_starpaths, update_starpath,
        update_starpath_chapter, update_starpath_content_status_admin, update_starpath_lab,
    },
};

pub mod health;
pub mod starpath_feedback;
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
        .route(
            "/starpaths/{id}/feedbacks",
            get(get_starpath_feedbacks).post(create_starpath_feedback),
        )
        .route(
            "/starpaths/{id}/feedback-summary",
            get(get_starpath_feedback_summary),
        )
        .route(
            "/starpaths/{id}/engagement-summary",
            get(get_starpath_engagement_summary),
        )
        .route("/starpaths/{id}/analytics", get(get_starpath_analytics))
        .route(
            "/starpaths/feedbacks/{feedback_id}",
            put(update_starpath_feedback).delete(delete_starpath_feedback),
        )
        .route(
            "/starpaths/feedbacks/{feedback_id}/vote",
            put(set_starpath_feedback_vote),
        )
        .route(
            "/starpaths/feedbacks/{feedback_id}/reply",
            put(upsert_starpath_feedback_reply).delete(delete_starpath_feedback_reply),
        )
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
            "/starpaths/{id}/chapters",
            get(get_starpath_chapters).post(create_starpath_chapter),
        )
        .route(
            "/starpaths/{id}/chapters/{chapter_id}",
            put(update_starpath_chapter).delete(delete_starpath_chapter),
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
