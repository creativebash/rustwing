use crate::domain::post::Post;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct CreatePost {
    pub user_id: Uuid,
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    pub body: String,
}

#[derive(Deserialize)]
pub struct UpdatePost {
    pub user_id: Option<Uuid>,
    pub title: Option<String>,
    pub body: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct PostResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Post> for PostResponse {
    fn from(model: Post) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            title: model.title,
            body: model.body,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
