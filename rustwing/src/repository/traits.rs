// src/repository/traits.rs
use uuid::Uuid;

pub trait ModelName {
    fn table_name() -> &'static str;
}

pub trait Insertable {
    fn generate_id() -> Option<Uuid> {
        Some(Uuid::now_v7()) // default: UUID v7, override to return None to opt out
    }
    fn columns() -> Vec<&'static str>;
    fn bind_values<'a>(&'a self, query: &mut sqlx::QueryBuilder<'a, sqlx::Postgres>);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateResult {
    HasUpdates,
    NoChanges,
}

pub trait Updateable {
    fn bind_updates<'a>(
        &'a self,
        query: &mut sqlx::QueryBuilder<'a, sqlx::Postgres>,
    ) -> UpdateResult;
}

// // src/repository/traits.rs
// pub trait ModelName {
//     fn table_name() -> &'static str;
// }

// pub trait Insertable {
//     fn columns() -> Vec<&'static str>;
//     fn bind_values<'a>(&'a self, query: &mut sqlx::QueryBuilder<'a, sqlx::Postgres>);
// }

// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
// pub enum UpdateResult {
//     HasUpdates,
//     NoChanges,
// }

// pub trait Updateable {
//     fn bind_updates<'a>(&'a self, query: &mut sqlx::QueryBuilder<'a, sqlx::Postgres>) -> UpdateResult;
// }
