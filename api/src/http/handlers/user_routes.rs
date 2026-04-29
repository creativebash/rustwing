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
    http::dtos::user_dto::{CreateUser, UpdateUser, UserResponse},
    http::extractors::AuthUser,
    repository::user_repo::UserUpdate,
    services::user_service::UserService,
    state::AppState,
};
use rustwing::prelude::*;

#[derive(Deserialize)]
pub struct Pagination {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct CursorPagination {
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}

pub async fn list_users(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<Pagination>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let users = generic_crud::find_all::<User>(
        &state.db,
        params.limit.unwrap_or(20),
        params.offset.unwrap_or(0),
    )
    .await?;
    Ok(Json(users.into_iter().map(UserResponse::from).collect()))
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

pub async fn create_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> Result<(StatusCode, Json<UserResponse>), AppError> {
    payload.validate()?;

    let user = UserService::create_user_with_bio(&state.db, &state.llm, payload).await?;

    Ok((StatusCode::CREATED, Json(UserResponse::from(user))))
}

pub async fn update_user(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUser>,
) -> Result<Json<UserResponse>, AppError> {
    let update = UserUpdate::from(payload);
    let user = generic_crud::update::<User, UserUpdate>(&state.db, id, &update).await?;
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
