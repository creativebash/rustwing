use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;
use validator::Validate;

use crate::{
    domain::post::Post,
    error::AppError,
    http::dtos::post_dto::{CreatePost, PostResponse, UpdatePost},
    http::extractors::AuthUser,
    http::handlers::user_routes::{CursorPagination, Pagination},
    repository::post_repo::{InsertPost, PostUpdate},
    state::AppState,
};
use rustwing::repository::generic_crud;

pub async fn list_posts(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<PostResponse>>, AppError> {
    let items =
        generic_crud::find_all::<Post>(&state.db, p.limit.unwrap_or(10), p.offset.unwrap_or(0))
            .await?;
    Ok(Json(items.into_iter().map(PostResponse::from).collect()))
}

pub async fn list_posts_cursor(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(p): Query<CursorPagination>,
) -> Result<Json<Vec<PostResponse>>, AppError> {
    let after = p.after.unwrap_or_else(Uuid::nil);
    let items = generic_crud::find_after::<Post>(&state.db, after, p.limit.unwrap_or(10)).await?;
    Ok(Json(items.into_iter().map(PostResponse::from).collect()))
}

pub async fn get_post(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PostResponse>, AppError> {
    let item = generic_crud::find_by_id::<Post>(&state.db, id).await?;
    Ok(Json(PostResponse::from(item)))
}

pub async fn create_post(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreatePost>,
) -> Result<(StatusCode, Json<PostResponse>), AppError> {
    payload.validate()?;
    let insert = InsertPost::from(payload);
    let item = generic_crud::insert::<Post, InsertPost>(&state.db, &insert).await?;
    Ok((StatusCode::CREATED, Json(PostResponse::from(item))))
}

pub async fn update_post(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdatePost>,
) -> Result<Json<PostResponse>, AppError> {
    let update = PostUpdate::from(payload);
    let item = generic_crud::update::<Post, PostUpdate>(&state.db, id, &update).await?;
    Ok(Json(PostResponse::from(item)))
}

pub async fn delete_post(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    generic_crud::delete::<Post>(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
