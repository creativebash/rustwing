use crate::domain::user::User;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    #[validate(length(min = 1, max = 50))]
    pub username: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Validate, ToSchema)]
#[schema(as = UpdateUserRequest)]
pub struct UpdateUser {
    #[validate(length(min = 1, max = 50))]
    pub username: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(max = 2000))]
    pub bio: Option<String>,
}

#[derive(Serialize, ToSchema, Clone)]
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

#[derive(Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}
