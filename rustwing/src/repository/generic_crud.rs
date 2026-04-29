// src/repository/generic_crud.rs
use crate::{error::CoreError, repository::traits::*};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

pub async fn find_all<T>(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<T>, CoreError>
where
    T: ModelName + for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let query = format!("SELECT * FROM {} LIMIT $1 OFFSET $2", T::table_name());
    let records = sqlx::query_as(&query)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok(records)
}

pub async fn find_after<T>(pool: &PgPool, after_id: Uuid, limit: i64) -> Result<Vec<T>, CoreError>
where
    T: ModelName + for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let query = format!(
        "SELECT * FROM {} WHERE id > $1 ORDER BY id LIMIT $2",
        T::table_name()
    );
    let records = sqlx::query_as(&query)
        .bind(after_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(records)
}

pub async fn find_by_id<T>(pool: &PgPool, id: Uuid) -> Result<T, CoreError>
where
    T: ModelName + for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let query = format!("SELECT * FROM {} WHERE id = $1", T::table_name());
    sqlx::query_as(&query)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(CoreError::NotFound)
}

pub async fn insert<T, I>(pool: &PgPool, data: &I) -> Result<T, CoreError>
where
    T: ModelName + for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    I: Insertable,
{
    let mut qb = sqlx::QueryBuilder::new(format!("INSERT INTO {} (", T::table_name()));

    let mut all_columns = I::columns();
    let generated_id = I::generate_id();

    // prepend "id" column if generate_id() returns Some
    if generated_id.is_some() {
        all_columns.insert(0, "id");
    }

    qb.push(all_columns.join(", ")).push(") VALUES (");

    // bind id first if generated
    if let Some(id) = generated_id {
        qb.push_bind(id);
        if !I::columns().is_empty() {
            qb.push(", ");
        }
    }

    data.bind_values(&mut qb);
    qb.push(") RETURNING *");

    Ok(qb.build_query_as().fetch_one(pool).await?)
}

pub async fn update<T, U>(pool: &PgPool, id: Uuid, data: &U) -> Result<T, CoreError>
where
    T: ModelName + for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    U: Updateable,
{
    let mut qb = sqlx::QueryBuilder::new(format!("UPDATE {} SET ", T::table_name()));
    if data.bind_updates(&mut qb) == UpdateResult::NoChanges {
        return find_by_id(pool, id).await;
    }
    qb.push(" WHERE id = ").push_bind(id).push(" RETURNING *");
    qb.build_query_as()
        .fetch_optional(pool)
        .await?
        .ok_or(CoreError::NotFound)
}

pub async fn delete<T: ModelName>(pool: &PgPool, id: Uuid) -> Result<(), CoreError> {
    let query = format!("DELETE FROM {} WHERE id = $1", T::table_name());
    let result = sqlx::query(&query).bind(id).execute(pool).await?;
    if result.rows_affected() == 0 {
        Err(CoreError::NotFound)
    } else {
        Ok(())
    }
}
