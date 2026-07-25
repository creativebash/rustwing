use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow, ToSchema, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub credit_balance: i32,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
