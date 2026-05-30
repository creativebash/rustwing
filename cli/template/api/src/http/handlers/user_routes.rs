use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{
    error::{AppError, ErrorResponse},
    http::dtos::user_dto::{UpdateUser, UserResponse},
    http::extractors::AuthUser,
    services::user_service,
    state::AppState,
};

#[allow(dead_code)]
#[derive(Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct CursorPagination {
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/users/cursor",
    tag = "Users",
    operation_id = "listUsersCursor",
    params(CursorPagination),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "Users returned", body = Vec<UserResponse>),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn list_users_cursor(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<CursorPagination>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let users = user_service::list_users_cursor(&state.db, params.after, params.limit).await?;
    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

#[utoipa::path(
    get,
    path = "/users/{id}",
    tag = "Users",
    operation_id = "getUser",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    security(("bearerAuth" = [])),
    responses(
        (status = 200, description = "User returned", body = UserResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn get_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, AppError> {
    let user = user_service::get_user(&state.db, id).await?;
    Ok(Json(UserResponse::from(user)))
}

#[utoipa::path(
    patch,
    path = "/users/{id}",
    tag = "Users",
    operation_id = "updateUser",
    request_body = UpdateUser,
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
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
pub async fn update_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<UserResponse>, AppError> {
    let user = user_service::update_user(&state.db, id, payload).await?;
    Ok(Json(UserResponse::from(user)))
}

#[utoipa::path(
    delete,
    path = "/users/{id}",
    tag = "Users",
    operation_id = "deleteUser",
    params(
        ("id" = Uuid, Path, description = "User ID")
    ),
    security(("bearerAuth" = [])),
    responses(
        (status = 204, description = "User deleted"),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
pub async fn delete_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    user_service::delete_user(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
