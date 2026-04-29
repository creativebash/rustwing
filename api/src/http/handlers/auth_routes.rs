use axum::{Json, extract::State, http::StatusCode};
use rustwing::prelude::*;
use validator::Validate;

use crate::{
    domain::user::User,
    error::AppError,
    http::dtos::user_dto::{AuthResponse, LoginRequest, RegisterRequest, UserResponse},
    state::AppState,
};

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    payload.validate()?;

    // 1. Hash the password
    let password_hash = AuthEngine::hash_password(&payload.password)?;

    // 2. We skip using Generic CRUD here because we need a custom query to insert the password
    let query = "INSERT INTO users (username, email, password_hash, credit_balance) VALUES ($1, $2, $3, 0) RETURNING *";
    let user: User = sqlx::query_as(query)
        .bind(&payload.username)
        .bind(&payload.email)
        .bind(&password_hash)
        .fetch_one(&state.db)
        .await?;

    // 3. Generate JWT
    let token = AuthEngine::create_jwt(user.id, &state.jwt_secret)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: UserResponse::from(user),
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // 1. Find user by email
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&state.db)
        .await?;

    let user = user.ok_or(AppError::Core(rustwing::error::CoreError::NotFound))?;

    // 2. Verify password (assuming your User domain struct has a password_hash field now)
    if !AuthEngine::verify_password(&payload.password, &user.password_hash) {
        return Err(AppError::Core(rustwing::error::CoreError::NotFound)); // Generic error to prevent email enumeration
    }

    // 3. Generate JWT
    let token = AuthEngine::create_jwt(user.id, &state.jwt_secret)?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse::from(user),
    }))
}
