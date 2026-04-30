pub mod dtos;
pub mod extractors;
pub mod handlers;

use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::root::health))
        .route("/auth/register", post(handlers::auth_routes::register))
        .route("/auth/login", post(handlers::auth_routes::login))
        .route("/users/cursor", get(handlers::user_routes::list_users_cursor))
        .route(
            "/users/{id}",
            get(handlers::user_routes::get_user)
                .put(handlers::user_routes::update_user)
                .delete(handlers::user_routes::delete_user),
        )
        .with_state(state)
}
