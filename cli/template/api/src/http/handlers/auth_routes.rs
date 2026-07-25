use axum::{Json, extract::State, http::StatusCode};

use crate::{
    error::{AppError, ErrorResponse},
    http::dtos::user_dto::{AuthResponse, LoginRequest, RegisterRequest},
    services::auth_service,
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
    let response = auth_service::register(&state.db, &state.jwt_secret, payload).await?;
    Ok((StatusCode::CREATED, Json(response)))
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
    Ok(Json(
        auth_service::login(&state.db, &state.jwt_secret, payload).await?,
    ))
}
