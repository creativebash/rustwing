// src/api/dtos/user_dto.rs
use crate::domain::user::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct CreateUser {
    #[validate(length(min = 1, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(range(min = 0))]
    pub credit_balance: i32,
    pub bio: Option<String>,
}

impl CreateUser {
    pub fn validate(&self) -> Result<(), validator::ValidationErrors> {
        validator::Validate::validate(self)
    }
}

#[derive(Deserialize)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub email: Option<String>,
    pub credit_balance: Option<i32>,
    pub bio: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub credit_balance: i32,
    pub bio: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(model: User) -> Self {
        Self {
            id: model.id,
            username: model.username,
            email: model.email,
            credit_balance: model.credit_balance,
            bio: model.bio,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

#[derive(Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 1, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}
