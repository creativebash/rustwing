use rustwing::prelude::*;
use sqlx::PgPool;
use tokio::task;
use validator::Validate;

use crate::{
    domain::user::User,
    error::AppError,
    http::dtos::user_dto::{AuthResponse, LoginRequest, RegisterRequest, UserResponse},
    repository::user_repo::UserRecord,
};

pub async fn register(
    db: &PgPool,
    jwt_secret: &str,
    payload: RegisterRequest,
) -> Result<AuthResponse, AppError> {
    payload.validate()?;

    let RegisterRequest {
        username,
        email,
        password,
    } = payload;
    let password_hash = task::spawn_blocking(move || AuthEngine::hash_password(&password))
        .await
        .map_err(|error| CoreError::Internal(format!("Password task failed: {error}")))??;

    let record: UserRecord = sqlx::query_as(
        "INSERT INTO users (username, email, password_hash, credit_balance) \
         VALUES ($1, $2, $3, 0) RETURNING *",
    )
    .bind(&username)
    .bind(&email)
    .bind(&password_hash)
    .fetch_one(db)
    .await?;

    let token = AuthEngine::create_jwt(record.id, jwt_secret)?;

    let user: User = record.into();
    Ok(AuthResponse {
        token,
        user: UserResponse::from(user),
    })
}

pub async fn login(
    db: &PgPool,
    jwt_secret: &str,
    payload: LoginRequest,
) -> Result<AuthResponse, AppError> {
    let LoginRequest { email, password } = payload;
    let record: Option<UserRecord> = sqlx::query_as("SELECT * FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(db)
        .await?;

    // Always perform expensive password work. For an unknown account, hashing
    // the supplied password reduces the login timing difference without
    // retaining a shared dummy password hash.
    let password_hash = record.as_ref().map(|user| user.password_hash.clone());
    let password_valid = task::spawn_blocking(move || match password_hash {
        Some(hash) => AuthEngine::verify_password(&password, &hash),
        None => {
            let _ = AuthEngine::hash_password(&password);
            false
        }
    })
    .await
    .map_err(|error| CoreError::Internal(format!("Password task failed: {error}")))?;

    let record = record.ok_or(CoreError::Unauthorized)?;
    if !password_valid {
        return Err(CoreError::Unauthorized.into());
    }

    let token = AuthEngine::create_jwt(record.id, jwt_secret)?;

    let user: User = record.into();
    Ok(AuthResponse {
        token,
        user: UserResponse::from(user),
    })
}
