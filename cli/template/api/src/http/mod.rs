pub mod dtos;
pub mod extractors;
pub mod handlers;
pub mod pagination;

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
        .route(
            "/users/me",
            get(handlers::user_routes::get_current_user)
                .put(handlers::user_routes::update_current_user)
                .patch(handlers::user_routes::update_current_user)
                .delete(handlers::user_routes::delete_current_user),
        )
        // rustwing:routes
        .merge(SwaggerUi::new("/docs").url("/openapi.json", crate::openapi::openapi()))
        .merge(Redoc::with_url("/redoc", crate::openapi::openapi()))
        .with_state(state)
}
