use axum::{Json, extract::State, http::StatusCode};
use rustwing::prelude::*;
use validator::Validate;

use crate::{
    domain::user::User,
    error::{AppError, ErrorResponse},
    http::dtos::user_dto::{AuthResponse, LoginRequest, RegisterRequest, UserResponse},
    state::AppState,
};

#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "Auth",
    operation_id = "register",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = AuthResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 409, description = "User already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    payload.validate()?;

    let password_hash = AuthEngine::hash_password(&payload.password)?;

    let query = "INSERT INTO users (username, email, password_hash, credit_balance) VALUES ($1, $2, $3, 0) RETURNING *";
    let user: User = sqlx::query_as(query)
        .bind(&payload.username)
        .bind(&payload.email)
        .bind(&password_hash)
        .fetch_one(&state.db)
        .await?;

    let token = AuthEngine::create_jwt(user.id, &state.jwt_secret)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            token,
            user: UserResponse::from(user),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "Auth",
    operation_id = "login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "User logged in", body = AuthResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&state.db)
        .await?;

    let user = user.ok_or(AppError::Core(CoreError::Unauthorized))?;

    if !AuthEngine::verify_password(&payload.password, &user.password_hash) {
        return Err(AppError::Core(CoreError::Unauthorized));
    }

    let token = AuthEngine::create_jwt(user.id, &state.jwt_secret)?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse::from(user),
    }))
}
