use crate::error::CoreError;
use chrono::{DateTime, Duration, Utc};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sqlx::{Acquire, FromRow, PgConnection, PgPool};
use std::{future::Future, pin::Pin};
use uuid::Uuid;

pub type IdempotentFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, CoreError>> + Send + 'a>>;

#[derive(Debug, Clone)]
pub struct IdempotencyScope {
    pub namespace: String,
    pub organisation_id: Option<Uuid>,
}

impl IdempotencyScope {
    pub fn global(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            organisation_id: None,
        }
    }
    pub fn tenant(namespace: impl Into<String>, organisation_id: Uuid) -> Self {
        Self {
            namespace: namespace.into(),
            organisation_id: Some(organisation_id),
        }
    }
    fn scope_key(&self) -> String {
        self.organisation_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "global".into())
    }
}

#[derive(Debug, Clone)]
pub struct IdempotencyOptions {
    pub retry_delay: Duration,
}

impl Default for IdempotencyOptions {
    fn default() -> Self {
        Self {
            retry_delay: Duration::seconds(5),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyOutcome<T> {
    Executed(T),
    Replayed(T),
}

#[derive(FromRow)]
struct Record {
    request_fingerprint: String,
    status: String,
    response: Option<Value>,
    retry_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct IdempotencyRecord {
    pub id: Uuid,
    pub namespace: String,
    pub organisation_id: Option<Uuid>,
    pub idempotency_key: String,
    pub request_fingerprint: String,
    pub status: String,
    pub attempts: i32,
    pub response: Option<Value>,
    pub last_error: Option<String>,
    pub retry_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct IdempotencyStore {
    pool: PgPool,
}

impl IdempotencyStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn inspect(
        &self,
        scope: &IdempotencyScope,
        key: &str,
    ) -> Result<Option<IdempotencyRecord>, CoreError> {
        Ok(sqlx::query_as("SELECT id, namespace, organisation_id, idempotency_key, request_fingerprint, status, attempts, response, last_error, retry_at, created_at, updated_at, completed_at FROM idempotency_records WHERE namespace=$1 AND scope_key=$2 AND idempotency_key=$3")
            .bind(&scope.namespace).bind(scope.scope_key()).bind(key).fetch_optional(&self.pool).await?)
    }

    pub async fn process_once<T, F>(
        &self,
        scope: IdempotencyScope,
        key: &str,
        request_fingerprint: &str,
        options: IdempotencyOptions,
        operation: F,
    ) -> Result<IdempotencyOutcome<T>, CoreError>
    where
        T: Serialize + DeserializeOwned + Send,
        F: for<'a> FnOnce(&'a mut PgConnection) -> IdempotentFuture<'a, T> + Send,
    {
        if scope.namespace.trim().is_empty()
            || key.trim().is_empty()
            || request_fingerprint.trim().is_empty()
        {
            return Err(CoreError::InvalidInput(
                "idempotency namespace, key, and fingerprint are required".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let id = Uuid::now_v7();
        let scope_key = scope.scope_key();
        sqlx::query("INSERT INTO idempotency_records (id, namespace, scope_key, organisation_id, idempotency_key, request_fingerprint, status, attempts) VALUES ($1,$2,$3,$4,$5,$6,'PROCESSING',0) ON CONFLICT (namespace, scope_key, idempotency_key) DO NOTHING")
            .bind(id).bind(&scope.namespace).bind(&scope_key).bind(scope.organisation_id).bind(key).bind(request_fingerprint).execute(&mut *tx).await?;

        let record: Record = sqlx::query_as("SELECT request_fingerprint, status, response, retry_at FROM idempotency_records WHERE namespace=$1 AND scope_key=$2 AND idempotency_key=$3 FOR UPDATE")
            .bind(&scope.namespace).bind(&scope_key).bind(key).fetch_one(&mut *tx).await?;
        if record.request_fingerprint != request_fingerprint {
            return Err(CoreError::Conflict(
                "idempotency key was already used for a different request".into(),
            ));
        }
        if record.status == "SUCCEEDED" {
            let value = record
                .response
                .ok_or_else(|| CoreError::Internal("idempotency result is missing".into()))?;
            let result = serde_json::from_value(value).map_err(|error| {
                CoreError::Internal(format!("invalid stored idempotency result: {error}"))
            })?;
            tx.commit().await?;
            return Ok(IdempotencyOutcome::Replayed(result));
        }
        if record.status == "FAILED"
            && record
                .retry_at
                .is_some_and(|retry_at| retry_at > Utc::now())
        {
            return Err(CoreError::Conflict(
                "idempotent operation is waiting for its retry window".into(),
            ));
        }
        sqlx::query("UPDATE idempotency_records SET status='PROCESSING', attempts=attempts+1, last_error=NULL, retry_at=NULL, updated_at=NOW() WHERE namespace=$1 AND scope_key=$2 AND idempotency_key=$3")
            .bind(&scope.namespace).bind(&scope_key).bind(key).execute(&mut *tx).await?;

        let mut savepoint = (&mut tx).begin().await?;
        match operation(&mut savepoint).await {
            Ok(result) => {
                let response = serde_json::to_value(&result).map_err(|error| {
                    CoreError::Internal(format!("failed to store idempotency result: {error}"))
                })?;
                sqlx::query("UPDATE idempotency_records SET status='SUCCEEDED', response=$4, completed_at=NOW(), updated_at=NOW() WHERE namespace=$1 AND scope_key=$2 AND idempotency_key=$3")
                    .bind(&scope.namespace).bind(&scope_key).bind(key).bind(response).execute(&mut *savepoint).await?;
                savepoint.commit().await?;
                tx.commit().await?;
                Ok(IdempotencyOutcome::Executed(result))
            }
            Err(error) => {
                savepoint.rollback().await?;
                let retry_at = Utc::now() + options.retry_delay;
                let message: String = error.to_string().chars().take(4_000).collect();
                sqlx::query("UPDATE idempotency_records SET status='FAILED', last_error=$4, retry_at=$5, updated_at=NOW() WHERE namespace=$1 AND scope_key=$2 AND idempotency_key=$3")
                    .bind(&scope.namespace).bind(&scope_key).bind(key).bind(message).bind(retry_at).execute(&mut *tx).await?;
                tx.commit().await?;
                Err(error)
            }
        }
    }
}
