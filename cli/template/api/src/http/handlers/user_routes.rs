use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    error::AppError,
    http::dtos::user_dto::{UpdateUser, UserResponse},
    http::extractors::AuthUser,
    services::user_service,
    state::AppState,
};

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Pagination {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct CursorPagination {
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}

pub async fn list_users_cursor(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<CursorPagination>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let users = user_service::list_users_cursor(&state.db, params.after, params.limit).await?;
    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

pub async fn get_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, AppError> {
    let user = user_service::get_user(&state.db, id).await?;
    Ok(Json(UserResponse::from(user)))
}

pub async fn update_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<UserResponse>, AppError> {
    let user = user_service::update_user(&state.db, id, payload).await?;
    Ok(Json(UserResponse::from(user)))
}

pub async fn delete_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    user_service::delete_user(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
