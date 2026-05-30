pub mod dtos;
pub mod extractors;
pub mod handlers;

use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

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
                .patch(handlers::user_routes::update_user)
                .delete(handlers::user_routes::delete_user),
        )
        // rustwing:routes
        .merge(SwaggerUi::new("/docs").url("/openapi.json", crate::openapi::openapi()))
        .merge(Redoc::with_url("/redoc", crate::openapi::openapi()))
        .with_state(state)
}
