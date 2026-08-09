pub mod dtos;
pub mod extractors;
pub mod handlers;
pub mod pagination;
pub mod rate_limit;
pub mod request_context;

use crate::state::AppState;
use axum::{
    Router, middleware,
    routing::{get, post},
};
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

pub fn app_router(state: AppState) -> Router {
    let global_limiter =
        rate_limit::RateLimiter::new(state.rate_limit.global_requests, state.rate_limit.window);
    let auth_limiter =
        rate_limit::RateLimiter::new(state.rate_limit.auth_requests, state.rate_limit.window);
    let auth_routes = Router::new()
        .route("/auth/register", post(handlers::auth_routes::register))
        .route("/auth/login", post(handlers::auth_routes::login))
        .layer(middleware::from_fn_with_state(
            auth_limiter,
            rate_limit::enforce,
        ));
    Router::new()
        .route("/health/live", get(handlers::root::live))
        .route("/health/ready", get(handlers::root::ready))
        .merge(auth_routes)
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
        .layer(middleware::from_fn_with_state(
            global_limiter,
            rate_limit::enforce,
        ))
        .layer(middleware::from_fn(request_context::attach))
        .with_state(state)
}
