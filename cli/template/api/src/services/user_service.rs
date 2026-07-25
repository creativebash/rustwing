use rustwing::prelude::*;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::{domain::user::User, error::AppError, http::dtos::user_dto::UpdateUser};

pub async fn get_user(db: &PgPool, id: Uuid) -> Result<User, AppError> {
    Ok(generic_crud::find_by_id::<User>(db, id).await?)
}

pub async fn update_user(db: &PgPool, id: Uuid, payload: UpdateUser) -> Result<User, AppError> {
    payload.validate()?;
    let query = "UPDATE users SET username = COALESCE($1, username), email = COALESCE($2, email), bio = COALESCE($3, bio) WHERE id = $4 RETURNING *";
    let user = sqlx::query_as(query)
        .bind(&payload.username)
        .bind(&payload.email)
        .bind(&payload.bio)
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or(CoreError::NotFound)?;

    Ok(user)
}

pub async fn delete_user(db: &PgPool, id: Uuid) -> Result<(), AppError> {
    Ok(generic_crud::delete::<User>(db, id).await?)
}
