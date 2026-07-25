use chrono::{DateTime, Utc};
use rustwing::prelude::*;
use sqlx::FromRow;
use uuid::Uuid;

use crate::domain::user::User;

impl ModelName for User {
    fn table_name() -> &'static str {
        "users"
    }
}

/// Database-only authentication record.
///
/// Keeping the password hash out of `User` prevents it from being exposed by
/// domain serialization, OpenAPI schemas, or response conversions.
#[derive(Debug, FromRow)]
pub(crate) struct UserRecord {
    pub id: Uuid,
    pub password_hash: String,
    pub username: String,
    pub email: String,
    pub credit_balance: i32,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<UserRecord> for User {
    fn from(record: UserRecord) -> Self {
        Self {
            id: record.id,
            username: record.username,
            email: record.email,
            credit_balance: record.credit_balance,
            bio: record.bio,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}
