use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use crate::{
    domain::post::Post,
    http::dtos::post_dto::{CreatePost, UpdatePost},
};
use rustwing::repository::traits::{Insertable, ModelName, UpdateResult, Updateable};

impl ModelName for Post {
    fn table_name() -> &'static str {
        "posts"
    }
}

pub struct InsertPost {
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
}

impl From<CreatePost> for InsertPost {
    fn from(dto: CreatePost) -> Self {
        Self {
            user_id: dto.user_id,
            title: dto.title,
            body: dto.body,
        }
    }
}

impl Insertable for InsertPost {
    fn columns() -> Vec<&'static str> {
        vec!["user_id", "title", "body"]
    }
    fn bind_values<'a>(&'a self, query: &mut QueryBuilder<'a, Postgres>) {
        let mut separated = query.separated(", ");
        separated.push_bind(&self.user_id);
        separated.push_bind(&self.title);
        separated.push_bind(&self.body);
    }
}

pub struct PostUpdate {
    pub user_id: Option<Uuid>,
    pub title: Option<String>,
    pub body: Option<String>,
}

impl From<UpdatePost> for PostUpdate {
    fn from(dto: UpdatePost) -> Self {
        Self {
            user_id: dto.user_id,
            title: dto.title,
            body: dto.body,
        }
    }
}

impl Updateable for PostUpdate {
    fn bind_updates<'a>(&'a self, query: &mut QueryBuilder<'a, Postgres>) -> UpdateResult {
        let mut separated = query.separated(", ");
        let mut has_updates = false;

        if let Some(ref v) = self.user_id {
            separated.push("user_id = ").push_bind_unseparated(v);
            has_updates = true;
        }
        if let Some(ref v) = self.title {
            separated.push("title = ").push_bind_unseparated(v);
            has_updates = true;
        }
        if let Some(ref v) = self.body {
            separated.push("body = ").push_bind_unseparated(v);
            has_updates = true;
        }
        if has_updates {
            UpdateResult::HasUpdates
        } else {
            UpdateResult::NoChanges
        }
    }
}
