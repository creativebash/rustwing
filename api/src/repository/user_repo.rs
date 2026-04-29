use sqlx::{Postgres, QueryBuilder};

use crate::{
    domain::user::User,
    http::dtos::user_dto::{CreateUser, UpdateUser},
};
use rustwing::repository::traits::{Insertable, ModelName, UpdateResult, Updateable};

impl ModelName for User {
    fn table_name() -> &'static str {
        "users"
    }
}

pub struct InsertUser {
    pub username: String,
    pub email: String,
    pub credit_balance: i32,
    pub bio: Option<String>,
}

impl From<CreateUser> for InsertUser {
    fn from(dto: CreateUser) -> Self {
        Self {
            username: dto.username,
            email: dto.email,
            credit_balance: dto.credit_balance,
            bio: dto.bio,
        }
    }
}

impl Insertable for InsertUser {
    fn columns() -> Vec<&'static str> {
        vec!["username", "email", "credit_balance", "bio"]
    }
    fn bind_values<'a>(&'a self, query: &mut QueryBuilder<'a, Postgres>) {
        let mut separated = query.separated(", ");
        separated.push_bind(&self.username);
        separated.push_bind(&self.email);
        separated.push_bind(self.credit_balance);
        separated.push_bind(&self.bio);
    }
}

pub struct UserUpdate {
    pub username: Option<String>,
    pub email: Option<String>,
    pub credit_balance: Option<i32>,
    pub bio: Option<String>,
}

impl From<UpdateUser> for UserUpdate {
    fn from(dto: UpdateUser) -> Self {
        Self {
            username: dto.username,
            email: dto.email,
            credit_balance: dto.credit_balance,
            bio: dto.bio,
        }
    }
}

impl Updateable for UserUpdate {
    fn bind_updates<'a>(&'a self, query: &mut QueryBuilder<'a, Postgres>) -> UpdateResult {
        let mut separated = query.separated(", ");
        let mut has_updates = false;

        if let Some(ref u) = self.username {
            separated.push("username = ").push_bind_unseparated(u);
            has_updates = true;
        }
        if let Some(ref e) = self.email {
            separated.push("email = ").push_bind_unseparated(e);
            has_updates = true;
        }
        if let Some(c) = self.credit_balance {
            separated.push("credit_balance = ").push_bind_unseparated(c);
            has_updates = true;
        }
        if let Some(ref b) = self.bio {
            separated.push("bio = ").push_bind_unseparated(b);
            has_updates = true;
        }

        if has_updates {
            UpdateResult::HasUpdates
        } else {
            UpdateResult::NoChanges
        }
    }
}
