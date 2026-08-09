// src/repository/generic_crud.rs
use crate::{error::CoreError, repository::traits::*};
use sqlx::{AssertSqlSafe, Executor, FromRow, Postgres};
use uuid::Uuid;

pub async fn find_all<'e, T>(
    executor: impl Executor<'e, Database = Postgres>,
    limit: i64,
    offset: i64,
) -> Result<Vec<T>, CoreError>
where
    T: ModelName + for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let query = format!(
        "SELECT * FROM {} ORDER BY id LIMIT $1 OFFSET $2",
        T::table_name()
    );
    // `table_name()` is a framework-controlled identifier, never user input.
    // audited: `table_name()` is framework-controlled; safe to assert
    let q_static: &'static str = Box::leak(query.into_boxed_str());
    let records = sqlx::query_as(AssertSqlSafe(q_static))
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await?;
    Ok(records)
}

pub async fn find_after<'e, T>(
    executor: impl Executor<'e, Database = Postgres>,
    after_id: Uuid,
    limit: i64,
) -> Result<Vec<T>, CoreError>
where
    T: ModelName + for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let query = format!(
        "SELECT * FROM {} WHERE id > $1 ORDER BY id LIMIT $2",
        T::table_name()
    );
    // audited: `table_name()` is framework-controlled; safe to assert
    let q_static: &'static str = Box::leak(query.into_boxed_str());
    let records = sqlx::query_as(AssertSqlSafe(q_static))
        .bind(after_id)
        .bind(limit)
        .fetch_all(executor)
        .await?;
    Ok(records)
}

pub async fn find_by_id<'e, T>(
    executor: impl Executor<'e, Database = Postgres>,
    id: Uuid,
) -> Result<T, CoreError>
where
    T: ModelName + for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let query = format!("SELECT * FROM {} WHERE id = $1", T::table_name());
    // audited: `table_name()` is framework-controlled; safe to assert
    let q_static: &'static str = Box::leak(query.into_boxed_str());
    sqlx::query_as(AssertSqlSafe(q_static))
        .bind(id)
        .fetch_optional(executor)
        .await?
        .ok_or(CoreError::NotFound)
}

pub async fn insert<'e, T, I>(
    executor: impl Executor<'e, Database = Postgres>,
    data: &I,
) -> Result<T, CoreError>
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

    Ok(qb.build_query_as().fetch_one(executor).await?)
}

pub async fn update<'e, T, U>(
    executor: impl Executor<'e, Database = Postgres>,
    id: Uuid,
    data: &U,
) -> Result<T, CoreError>
where
    T: ModelName + for<'r> FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    U: Updateable,
{
    let mut qb = sqlx::QueryBuilder::new(format!("UPDATE {} SET ", T::table_name()));
    if data.bind_updates(&mut qb) == UpdateResult::NoChanges {
        return find_by_id(executor, id).await;
    }
    qb.push(" WHERE id = ").push_bind(id).push(" RETURNING *");
    qb.build_query_as()
        .fetch_optional(executor)
        .await?
        .ok_or(CoreError::NotFound)
}

pub async fn delete<'e, T: ModelName>(
    executor: impl Executor<'e, Database = Postgres>,
    id: Uuid,
) -> Result<(), CoreError> {
    let query = format!("DELETE FROM {} WHERE id = $1", T::table_name());
    // audited: `table_name()` is framework-controlled; safe to assert
    let q_static: &'static str = Box::leak(query.into_boxed_str());
    let result = sqlx::query(AssertSqlSafe(q_static))
        .bind(id)
        .execute(executor)
        .await?;
    if result.rows_affected() == 0 {
        Err(CoreError::NotFound)
    } else {
        Ok(())
    }
}
