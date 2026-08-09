use chrono::{Duration, Utc};
use rustwing::prelude::*;
use serde_json::json;
use sqlx::{AssertSqlSafe, FromRow, PgPool, Postgres, QueryBuilder, postgres::PgPoolOptions};
use std::{path::Path, sync::Arc};
use uuid::Uuid;

async fn test_pool() -> Option<(PgPool, PgPool, String)> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .ok()?;
    let schema = format!("rustwing_test_{}", Uuid::now_v7().simple());
    sqlx::query(AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .ok()?;
    let search_path = Arc::new(schema.clone());
    let pool = PgPoolOptions::new()
        .max_connections(12)
        .after_connect(move |connection, _| {
            let statement = format!("SET search_path TO {}", search_path);
            Box::pin(async move {
                sqlx::query(AssertSqlSafe(statement))
                    .execute(connection)
                    .await
                    .map(|_| ())
            })
        })
        .connect(&url)
        .await
        .ok()?;
    let migrator = sqlx::migrate::Migrator::new(Path::new("../cli/template/api/migrations"))
        .await
        .ok()?;
    migrator.run(&pool).await.ok()?;
    Some((pool, admin, schema))
}

#[tokio::test]
async fn postgres_reliability_and_isolation_suite() {
    let Some((pool, admin, schema)) = test_pool().await else {
        eprintln!("DATABASE_URL is not set or unavailable; PostgreSQL hardening suite skipped");
        return;
    };

    jobs_are_durable_and_leased(&pool).await;
    outbox_is_atomic_and_retriable(&pool).await;
    idempotency_is_concurrency_safe(&pool).await;
    tenant_queries_enforce_every_scope(&pool).await;
    uuid_cursor_pages_have_no_gaps(&pool).await;
    generic_repositories_share_transactions(&pool).await;

    pool.close().await;
    sqlx::query(AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await
        .unwrap();
}

#[derive(FromRow)]
struct TransactionItem {
    id: Uuid,
}
impl ModelName for TransactionItem {
    fn table_name() -> &'static str {
        "transaction_items"
    }
}
struct NewTransactionItem;
impl Insertable for NewTransactionItem {
    fn columns() -> Vec<&'static str> {
        vec![]
    }
    fn bind_values(&self, _query: &mut QueryBuilder<Postgres>) {}
}

async fn generic_repositories_share_transactions(pool: &PgPool) {
    sqlx::query("CREATE TABLE transaction_items (id UUID PRIMARY KEY)")
        .execute(pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    let first = generic_crud::insert::<TransactionItem, _>(&mut *tx, &NewTransactionItem)
        .await
        .unwrap();
    let found = generic_crud::find_by_id::<TransactionItem>(&mut *tx, first.id)
        .await
        .unwrap();
    assert_eq!(found.id, first.id);
    generic_crud::delete::<TransactionItem>(&mut *tx, first.id)
        .await
        .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM transaction_items")
            .fetch_one(pool)
            .await
            .unwrap(),
        0
    );
}

async fn jobs_are_durable_and_leased(pool: &PgPool) {
    let queue = JobQueue::new(pool.clone());
    let id = queue
        .enqueue(
            "test",
            &json!({"value": 1}),
            JobOptions {
                max_attempts: 2,
                correlation_id: Some("request-job-1".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let claimed = queue
        .claim("worker-a", 10, Duration::seconds(30))
        .await
        .unwrap();
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].id, id);
    assert_eq!(claimed[0].correlation_id.as_deref(), Some("request-job-1"));
    assert!(
        queue
            .claim("worker-b", 10, Duration::seconds(30))
            .await
            .unwrap()
            .is_empty()
    );
    queue
        .fail(
            "worker-a",
            &claimed[0],
            "temporary",
            true,
            &RetryPolicy {
                base_delay: Duration::zero(),
                max_delay: Duration::zero(),
            },
        )
        .await
        .unwrap();
    let retried = queue
        .claim("worker-b", 10, Duration::seconds(30))
        .await
        .unwrap();
    assert_eq!(retried[0].attempts, 2);
    queue.complete("worker-b", id).await.unwrap();

    let stale = queue
        .enqueue("stale", &json!({}), JobOptions::default())
        .await
        .unwrap();
    queue
        .claim("crashed", 1, Duration::seconds(30))
        .await
        .unwrap();
    sqlx::query("UPDATE jobs SET locked_at=NOW()-INTERVAL '2 hours' WHERE id=$1")
        .bind(stale)
        .execute(pool)
        .await
        .unwrap();
    assert_eq!(
        queue
            .claim("replacement", 1, Duration::seconds(30))
            .await
            .unwrap()[0]
            .id,
        stale
    );
}

async fn outbox_is_atomic_and_retriable(pool: &PgPool) {
    sqlx::query("CREATE TABLE business_events (id UUID PRIMARY KEY)")
        .execute(pool)
        .await
        .unwrap();
    let aggregate = Uuid::now_v7();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO business_events(id) VALUES($1)")
        .bind(aggregate)
        .execute(&mut *tx)
        .await
        .unwrap();
    let event_id = Outbox::record(
        &mut tx,
        NewOutboxEvent {
            event_type: "created".into(),
            aggregate_type: "test".into(),
            aggregate_id: aggregate,
            organisation_id: None,
            payload: json!({"id": aggregate}),
            correlation_id: Some("request-1".into()),
            available_at: Utc::now(),
            max_attempts: 3,
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let outbox = Outbox::new(pool.clone());
    let event = outbox
        .claim("dispatcher-a", 1, Duration::seconds(30))
        .await
        .unwrap()
        .remove(0);
    assert_eq!(event.id, event_id);
    assert!(
        outbox
            .claim("dispatcher-b", 1, Duration::seconds(30))
            .await
            .unwrap()
            .is_empty()
    );
    outbox
        .mark_failed(
            "dispatcher-a",
            &event,
            "temporary",
            true,
            &RetryPolicy {
                base_delay: Duration::zero(),
                max_delay: Duration::zero(),
            },
        )
        .await
        .unwrap();
    let event = outbox
        .claim("dispatcher-b", 1, Duration::seconds(30))
        .await
        .unwrap()
        .remove(0);
    outbox
        .mark_dispatched("dispatcher-b", event.id)
        .await
        .unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rolled_back = Uuid::now_v7();
    sqlx::query("INSERT INTO business_events(id) VALUES($1)")
        .bind(rolled_back)
        .execute(&mut *tx)
        .await
        .unwrap();
    Outbox::record(
        &mut tx,
        NewOutboxEvent {
            event_type: "rollback".into(),
            aggregate_type: "test".into(),
            aggregate_id: rolled_back,
            organisation_id: None,
            payload: json!({}),
            correlation_id: None,
            available_at: Utc::now(),
            max_attempts: 1,
        },
    )
    .await
    .unwrap();
    tx.rollback().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM business_events WHERE id=$1")
            .bind(rolled_back)
            .fetch_one(pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM outbox_events WHERE aggregate_id=$1")
            .bind(rolled_back)
            .fetch_one(pool)
            .await
            .unwrap(),
        0
    );
}

async fn idempotency_is_concurrency_safe(pool: &PgPool) {
    sqlx::query("CREATE TABLE effects (name TEXT PRIMARY KEY, count INTEGER NOT NULL)")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO effects VALUES ('once',0)")
        .execute(pool)
        .await
        .unwrap();
    let store = IdempotencyStore::new(pool.clone());
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let store = store.clone();
        tasks.push(tokio::spawn(async move {
            store.process_once(IdempotencyScope::global("provider"), "event-1", "hash-1", IdempotencyOptions { retry_delay: Duration::zero() }, |connection| Box::pin(async move {
                let count = sqlx::query_scalar::<_, i32>("UPDATE effects SET count=count+1 WHERE name='once' RETURNING count").fetch_one(connection).await?;
                Ok(count)
            })).await.unwrap()
        }));
    }
    for task in tasks {
        assert!(matches!(
            task.await.unwrap(),
            IdempotencyOutcome::Executed(1) | IdempotencyOutcome::Replayed(1)
        ));
    }
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT count FROM effects WHERE name='once'")
            .fetch_one(pool)
            .await
            .unwrap(),
        1
    );
    assert!(matches!(
        store
            .process_once(
                IdempotencyScope::global("provider"),
                "event-1",
                "different",
                Default::default(),
                |_| Box::pin(async { Ok(1_i32) })
            )
            .await,
        Err(CoreError::Conflict(_))
    ));

    let failed = store
        .process_once(
            IdempotencyScope::global("provider"),
            "retry",
            "hash",
            IdempotencyOptions {
                retry_delay: Duration::zero(),
            },
            |_| Box::pin(async { Err::<i32, _>(CoreError::Internal("temporary".into())) }),
        )
        .await;
    assert!(failed.is_err());
    assert!(matches!(
        store
            .process_once(
                IdempotencyScope::global("provider"),
                "retry",
                "hash",
                Default::default(),
                |_| Box::pin(async { Ok(7_i32) })
            )
            .await
            .unwrap(),
        IdempotencyOutcome::Executed(7)
    ));
}

async fn tenant_queries_enforce_every_scope(pool: &PgPool) {
    sqlx::query("CREATE TABLE scoped_resources (id UUID PRIMARY KEY, organisation_id UUID NOT NULL, parent_id UUID NOT NULL, value TEXT NOT NULL)").execute(pool).await.unwrap();
    let org_a = Uuid::now_v7();
    let org_b = Uuid::now_v7();
    let parent_a = Uuid::now_v7();
    let parent_b = Uuid::now_v7();
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO scoped_resources VALUES($1,$2,$3,'a')")
        .bind(id)
        .bind(org_a)
        .bind(parent_a)
        .execute(pool)
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM scoped_resources WHERE organisation_id=$1 AND parent_id=$2 AND id=$3"
        )
        .bind(org_b)
        .bind(parent_b)
        .bind(id)
        .fetch_optional(pool)
        .await
        .unwrap()
        .is_none()
    );
    assert_eq!(sqlx::query("UPDATE scoped_resources SET value='b' WHERE organisation_id=$1 AND parent_id=$2 AND id=$3").bind(org_b).bind(parent_b).bind(id).execute(pool).await.unwrap().rows_affected(), 0);
    assert_eq!(
        sqlx::query(
            "DELETE FROM scoped_resources WHERE organisation_id=$1 AND parent_id=$2 AND id=$3"
        )
        .bind(org_b)
        .bind(parent_b)
        .bind(id)
        .execute(pool)
        .await
        .unwrap()
        .rows_affected(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT value FROM scoped_resources WHERE organisation_id=$1 AND parent_id=$2 AND id=$3"
        )
        .bind(org_a)
        .bind(parent_a)
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap(),
        "a"
    );
}

async fn uuid_cursor_pages_have_no_gaps(pool: &PgPool) {
    sqlx::query("CREATE TABLE cursor_items (id UUID PRIMARY KEY)")
        .execute(pool)
        .await
        .unwrap();
    let ids: Vec<_> = (0..2_000).map(|_| Uuid::now_v7()).collect();
    for id in &ids {
        sqlx::query("INSERT INTO cursor_items(id) VALUES($1)")
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
    }
    let mut after = Uuid::nil();
    let mut seen = Vec::new();
    loop {
        let page: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM cursor_items WHERE id>$1 ORDER BY id LIMIT 37")
                .bind(after)
                .fetch_all(pool)
                .await
                .unwrap();
        if page.is_empty() {
            break;
        }
        after = *page.last().unwrap();
        seen.extend(page);
    }
    assert_eq!(seen, ids);
}
