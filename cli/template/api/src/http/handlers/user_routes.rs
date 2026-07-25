use axum::{Json, extract::State, http::StatusCode};

use crate::{
    error::{AppError, ErrorResponse},
    http::dtos::user_dto::{UpdateUser, UserResponse},
    http::extractors::AuthUser,
    services::user_service,
    state::AppState,
};

#[utoipa::path(
    get,
    path = "/users/me",
    tag = "Users",
    operation_id = "getCurrentUser",
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "User returned", body = UserResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_current_user(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<UserResponse>, AppError> {
    let user = user_service::get_user(&state.db, auth.id).await?;
    Ok(Json(UserResponse::from(user)))
}

#[utoipa::path(
    patch,
    path = "/users/me",
    tag = "Users",
    operation_id = "updateCurrentUser",
    request_body = UpdateUser,
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "User updated", body = UserResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 409, description = "Conflict", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn update_current_user(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<UserResponse>, AppError> {
    let user = user_service::update_user(&state.db, auth.id, payload).await?;
    Ok(Json(UserResponse::from(user)))
}

#[utoipa::path(
    delete,
    path = "/users/me",
    tag = "Users",
    operation_id = "deleteCurrentUser",
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "User deleted"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_current_user(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    user_service::delete_user(&state.db, auth.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
