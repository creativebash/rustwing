// src/http/mod.rs
// Axum routers, extractors, JSON DTOs
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
        .route("/auth/register", post(handlers::auth_routes::register))
        .route("/auth/login", post(handlers::auth_routes::login))
        .route(
            "/users",
            get(handlers::user_routes::list_users).post(handlers::user_routes::create_user),
        )
        .route(
            "/users/cursor",
            get(handlers::user_routes::list_users_cursor),
        )
        .route(
            "/users/{id}",
            get(handlers::user_routes::get_user)
                .put(handlers::user_routes::update_user)
                .delete(handlers::user_routes::delete_user),
        )
        .route("/ai/completions", post(handlers::ai_routes::complete))
        // Post routes
        .route(
            "/posts",
            get(handlers::post_routes::list_posts).post(handlers::post_routes::create_post),
        )
        .route(
            "/posts/cursor",
            get(handlers::post_routes::list_posts_cursor),
        )
        .route(
            "/posts/{id}",
            get(handlers::post_routes::get_post)
                .put(handlers::post_routes::update_post)
                .delete(handlers::post_routes::delete_post),
        )
        .with_state(state)
}
