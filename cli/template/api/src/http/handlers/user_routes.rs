use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    domain::user::User,
    error::AppError,
    http::dtos::user_dto::{UpdateUser, UserResponse},
    http::extractors::AuthUser,
    state::AppState,
};
use rustwing::prelude::*;

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
    let after = params.after.unwrap_or_else(Uuid::nil);
    let users =
        generic_crud::find_after::<User>(&state.db, after, params.limit.unwrap_or(10)).await?;
    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
}

pub async fn get_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserResponse>, AppError> {
    let user = generic_crud::find_by_id::<User>(&state.db, id).await?;
    Ok(Json(UserResponse::from(user)))
}

pub async fn update_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<UserResponse>, AppError> {
    let query = "UPDATE users SET username = COALESCE($1, username), email = COALESCE($2, email), bio = COALESCE($3, bio) WHERE id = $4 RETURNING *";
    let user: User = sqlx::query_as(query)
        .bind(&payload.username)
        .bind(&payload.email)
        .bind(&payload.bio)
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(UserResponse::from(user)))
}

pub async fn delete_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    generic_crud::delete::<User>(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
