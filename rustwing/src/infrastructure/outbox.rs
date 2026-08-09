use crate::{error::CoreError, infrastructure::jobs::RetryPolicy};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

pub struct NewOutboxEvent<T> {
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub organisation_id: Option<Uuid>,
    pub payload: T,
    pub correlation_id: Option<String>,
    pub available_at: DateTime<Utc>,
    pub max_attempts: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct OutboxEvent {
    pub id: Uuid,
    pub event_type: String,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub organisation_id: Option<Uuid>,
    pub payload: Value,
    pub correlation_id: Option<String>,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub available_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub dispatched_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct Outbox {
    pool: PgPool,
}

impl Outbox {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<OutboxEvent>, CoreError> {
        Ok(sqlx::query_as("SELECT * FROM outbox_events WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }

    pub async fn record<T: Serialize>(
        connection: &mut PgConnection,
        event: NewOutboxEvent<T>,
    ) -> Result<Uuid, CoreError> {
        if event.event_type.trim().is_empty() || event.aggregate_type.trim().is_empty() {
            return Err(CoreError::InvalidInput(
                "outbox event and aggregate types must not be empty".into(),
            ));
        }
        if event.max_attempts < 1 {
            return Err(CoreError::InvalidInput(
                "max_attempts must be positive".into(),
            ));
        }
        let id = Uuid::now_v7();
        let payload = serde_json::to_value(event.payload)
            .map_err(|error| CoreError::InvalidInput(format!("invalid outbox payload: {error}")))?;
        sqlx::query(
            "INSERT INTO outbox_events (id, event_type, aggregate_type, aggregate_id, organisation_id, payload, correlation_id, status, attempts, max_attempts, available_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'PENDING', 0, $8, $9)",
        )
        .bind(id).bind(event.event_type).bind(event.aggregate_type).bind(event.aggregate_id)
        .bind(event.organisation_id).bind(payload).bind(event.correlation_id)
        .bind(event.max_attempts).bind(event.available_at).execute(connection).await?;
        Ok(id)
    }

    pub async fn claim(
        &self,
        worker_id: &str,
        limit: i64,
        lease: Duration,
    ) -> Result<Vec<OutboxEvent>, CoreError> {
        let lease_seconds = lease.num_seconds().max(1);
        sqlx::query("UPDATE outbox_events SET status='DEAD', last_error=COALESCE(last_error, 'dispatcher lease expired after final attempt'), locked_at=NULL, locked_by=NULL WHERE status='RUNNING' AND attempts >= max_attempts AND locked_at < NOW() - ($1 * INTERVAL '1 second')")
            .bind(lease_seconds).execute(&self.pool).await?;
        let events = sqlx::query_as::<_, OutboxEvent>(
            "WITH candidates AS (SELECT id FROM outbox_events \
             WHERE attempts < max_attempts AND ((status IN ('PENDING','RETRY_SCHEDULED') AND available_at <= NOW()) \
             OR (status = 'RUNNING' AND locked_at < NOW() - ($3 * INTERVAL '1 second'))) \
             ORDER BY available_at, id FOR UPDATE SKIP LOCKED LIMIT $2) \
             UPDATE outbox_events AS event SET status='RUNNING', attempts=event.attempts+1, locked_at=NOW(), locked_by=$1 \
             FROM candidates WHERE event.id=candidates.id RETURNING event.*"
        ).bind(worker_id).bind(limit.clamp(1, 100)).bind(lease_seconds).fetch_all(&self.pool).await?;
        Ok(events)
    }

    pub async fn mark_dispatched(&self, worker_id: &str, id: Uuid) -> Result<(), CoreError> {
        let result = sqlx::query("UPDATE outbox_events SET status='DISPATCHED', dispatched_at=NOW(), locked_at=NULL, locked_by=NULL, last_error=NULL WHERE id=$1 AND status='RUNNING' AND locked_by=$2")
            .bind(id).bind(worker_id).execute(&self.pool).await?;
        if result.rows_affected() == 1 {
            Ok(())
        } else {
            Err(CoreError::Conflict(
                "outbox lease is no longer owned by this worker".into(),
            ))
        }
    }

    pub async fn heartbeat(&self, worker_id: &str, id: Uuid) -> Result<bool, CoreError> {
        let result = sqlx::query("UPDATE outbox_events SET locked_at=NOW() WHERE id=$1 AND status='RUNNING' AND locked_by=$2")
            .bind(id).bind(worker_id).execute(&self.pool).await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn mark_failed(
        &self,
        worker_id: &str,
        event: &OutboxEvent,
        error: &str,
        retryable: bool,
        policy: &RetryPolicy,
    ) -> Result<(), CoreError> {
        let status = if retryable && event.attempts < event.max_attempts {
            "RETRY_SCHEDULED"
        } else {
            "DEAD"
        };
        let available_at = Utc::now() + policy.delay_for_attempt(event.attempts);
        let error: String = error.chars().take(4_000).collect();
        let result = sqlx::query("UPDATE outbox_events SET status=$3, available_at=$4, last_error=$5, locked_at=NULL, locked_by=NULL WHERE id=$1 AND status='RUNNING' AND locked_by=$2")
            .bind(event.id).bind(worker_id).bind(status).bind(available_at).bind(error).execute(&self.pool).await?;
        if result.rows_affected() == 1 {
            Ok(())
        } else {
            Err(CoreError::Conflict(
                "outbox lease is no longer owned by this worker".into(),
            ))
        }
    }
}
