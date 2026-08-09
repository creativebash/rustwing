use rustwing::prelude::*;
use sqlx::PgPool;
use tokio::task;
use uuid::Uuid;
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

    let mut tx = db.begin().await?;
    let user_id = Uuid::now_v7();
    let record: UserRecord = sqlx::query_as(
        "INSERT INTO users (id, username, email, password_hash, credit_balance) \
         VALUES ($1, $2, $3, $4, 0) RETURNING *",
    )
    .bind(user_id)
    .bind(&username)
    .bind(&email)
    .bind(&password_hash)
    .fetch_one(&mut *tx)
    .await?;

    let organization_id = Uuid::now_v7();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, $2)")
        .bind(organization_id)
        .bind(format!("{}'s organization", username))
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO organization_members (organization_id, user_id, role) VALUES ($1, $2, 'owner')",
    )
    .bind(organization_id)
    .bind(record.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let token = AuthEngine::create_jwt(record.id, jwt_secret)?;

    let user: User = record.into();
    Ok(AuthResponse {
        token,
        user: UserResponse::from(user),
        organization_id,
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

    let organization_id: Uuid = sqlx::query_scalar(
        "SELECT organization_id FROM organization_members WHERE user_id = $1 AND status = 'active' ORDER BY created_at LIMIT 1",
    )
    .bind(record.id)
    .fetch_optional(db)
    .await?
    .ok_or(CoreError::Forbidden)?;

    let user: User = record.into();
    Ok(AuthResponse {
        token,
        user: UserResponse::from(user),
        organization_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::{AssertSqlSafe, postgres::PgPoolOptions};
    use std::sync::Arc;

    #[tokio::test]
    async fn registration_and_login_work_without_serializing_hashes() {
        let Ok(url) = std::env::var("DATABASE_URL") else {
            return;
        };
        let admin = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let schema = format!("auth_test_{}", Uuid::now_v7().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
            .execute(&admin)
            .await
            .unwrap();
        let search_path = Arc::new(schema.clone());
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .after_connect(move |connection, _| {
                let statement = format!("SET search_path TO {}", search_path);
                Box::pin(async move {
                    sqlx::query(AssertSqlSafe(statement))
                        .execute(connection)
                        .await
                        .map(|_| ())
                })
            })
            .connect(&url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let registered = register(
            &pool,
            "test-secret",
            RegisterRequest {
                username: "alice".into(),
                email: "alice@example.com".into(),
                password: "strong-password".into(),
            },
        )
        .await
        .unwrap();
        let logged_in = login(
            &pool,
            "test-secret",
            LoginRequest {
                email: "alice@example.com".into(),
                password: "strong-password".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(registered.user.id, logged_in.user.id);
        let json = serde_json::to_string(&registered.user).unwrap();
        assert!(!json.contains("password_hash"));
        assert!(!json.contains("argon2"));

        pool.close().await;
        sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
            .execute(&admin)
            .await
            .unwrap();
    }
}
