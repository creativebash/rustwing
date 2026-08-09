use crate::error::CoreError;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgConnection, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    RetryScheduled,
    Completed,
    Dead,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Running => "RUNNING",
            Self::RetryScheduled => "RETRY_SCHEDULED",
            Self::Completed => "COMPLETED",
            Self::Dead => "DEAD",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            base_delay: Duration::seconds(5),
            max_delay: Duration::hours(1),
        }
    }
}

impl RetryPolicy {
    pub fn delay_for_attempt(&self, attempt: i32) -> Duration {
        let exponent = attempt.saturating_sub(1).clamp(0, 30) as u32;
        let multiplier = 1_i64.checked_shl(exponent).unwrap_or(i64::MAX);
        let seconds = self
            .base_delay
            .num_seconds()
            .saturating_mul(multiplier)
            .min(self.max_delay.num_seconds())
            .max(0);
        Duration::seconds(seconds)
    }
}

#[derive(Debug, Clone)]
pub struct JobOptions {
    pub max_attempts: i32,
    pub available_at: DateTime<Utc>,
    pub correlation_id: Option<String>,
    pub organisation_id: Option<Uuid>,
}

impl Default for JobOptions {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            available_at: Utc::now(),
            correlation_id: None,
            organisation_id: None,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct ClaimedJob {
    pub id: Uuid,
    pub job_type: String,
    pub payload: Value,
    pub status: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub available_at: DateTime<Utc>,
    pub locked_at: Option<DateTime<Utc>>,
    pub locked_by: Option<String>,
    pub last_error: Option<String>,
    pub correlation_id: Option<String>,
    pub organisation_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl ClaimedJob {
    pub fn deserialize_payload<T: for<'de> Deserialize<'de>>(&self) -> Result<T, CoreError> {
        serde_json::from_value(self.payload.clone())
            .map_err(|error| CoreError::InvalidInput(format!("malformed job payload: {error}")))
    }
}

#[derive(Clone)]
pub struct JobQueue {
    pool: PgPool,
}

impl JobQueue {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn get(&self, id: Uuid) -> Result<Option<ClaimedJob>, CoreError> {
        Ok(sqlx::query_as("SELECT * FROM jobs WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?)
    }

    pub async fn enqueue<T: Serialize + ?Sized>(
        &self,
        job_type: &str,
        payload: &T,
        options: JobOptions,
    ) -> Result<Uuid, CoreError> {
        let mut connection = self.pool.acquire().await?;
        Self::enqueue_on(&mut connection, job_type, payload, options).await
    }

    pub async fn enqueue_on<T: Serialize + ?Sized>(
        connection: &mut PgConnection,
        job_type: &str,
        payload: &T,
        options: JobOptions,
    ) -> Result<Uuid, CoreError> {
        if job_type.trim().is_empty() {
            return Err(CoreError::InvalidInput("job_type must not be empty".into()));
        }
        if options.max_attempts < 1 {
            return Err(CoreError::InvalidInput(
                "max_attempts must be positive".into(),
            ));
        }
        let id = Uuid::now_v7();
        let payload = serde_json::to_value(payload)
            .map_err(|error| CoreError::InvalidInput(format!("invalid job payload: {error}")))?;
        sqlx::query(
            "INSERT INTO jobs (id, job_type, payload, status, attempts, max_attempts, available_at, correlation_id, organisation_id) \
             VALUES ($1, $2, $3, 'PENDING', 0, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(job_type)
        .bind(payload)
        .bind(options.max_attempts)
        .bind(options.available_at)
        .bind(options.correlation_id)
        .bind(options.organisation_id)
        .execute(connection)
        .await?;
        Ok(id)
    }

    pub async fn claim(
        &self,
        worker_id: &str,
        limit: i64,
        lease: Duration,
    ) -> Result<Vec<ClaimedJob>, CoreError> {
        let limit = limit.clamp(1, 100);
        let lease_seconds = lease.num_seconds().max(1);
        sqlx::query(
            "UPDATE jobs SET status='DEAD', last_error=COALESCE(last_error, 'worker lease expired after final attempt'), locked_at=NULL, locked_by=NULL, updated_at=NOW() \
             WHERE status='RUNNING' AND attempts >= max_attempts AND locked_at < NOW() - ($1 * INTERVAL '1 second')",
        )
        .bind(lease_seconds)
        .execute(&self.pool)
        .await?;
        let jobs = sqlx::query_as::<_, ClaimedJob>(
            "WITH candidates AS (\
                SELECT id FROM jobs \
                WHERE attempts < max_attempts AND (\
                    (status IN ('PENDING', 'RETRY_SCHEDULED') AND available_at <= NOW()) OR \
                    (status = 'RUNNING' AND locked_at < NOW() - ($3 * INTERVAL '1 second'))\
                ) \
                ORDER BY available_at, id \
                FOR UPDATE SKIP LOCKED LIMIT $2\
             ) \
             UPDATE jobs AS job SET \
                status = 'RUNNING', attempts = job.attempts + 1, locked_at = NOW(), \
                locked_by = $1, updated_at = NOW() \
             FROM candidates WHERE job.id = candidates.id \
             RETURNING job.*",
        )
        .bind(worker_id)
        .bind(limit)
        .bind(lease_seconds)
        .fetch_all(&self.pool)
        .await?;
        Ok(jobs)
    }

    pub async fn heartbeat(&self, worker_id: &str, job_id: Uuid) -> Result<bool, CoreError> {
        let result = sqlx::query(
            "UPDATE jobs SET locked_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND status = 'RUNNING' AND locked_by = $2",
        )
        .bind(job_id)
        .bind(worker_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() == 1)
    }

    pub async fn complete(&self, worker_id: &str, job_id: Uuid) -> Result<(), CoreError> {
        let result = sqlx::query(
            "UPDATE jobs SET status = 'COMPLETED', completed_at = NOW(), locked_at = NULL, \
             locked_by = NULL, last_error = NULL, updated_at = NOW() \
             WHERE id = $1 AND status = 'RUNNING' AND locked_by = $2",
        )
        .bind(job_id)
        .bind(worker_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 1 {
            Ok(())
        } else {
            Err(CoreError::Conflict(
                "job lease is no longer owned by this worker".into(),
            ))
        }
    }

    pub async fn fail(
        &self,
        worker_id: &str,
        job: &ClaimedJob,
        error: &str,
        retryable: bool,
        retry_policy: &RetryPolicy,
    ) -> Result<JobStatus, CoreError> {
        let exhausted = job.attempts >= job.max_attempts;
        let status = if retryable && !exhausted {
            JobStatus::RetryScheduled
        } else {
            JobStatus::Dead
        };
        let available_at = Utc::now() + retry_policy.delay_for_attempt(job.attempts);
        let error = truncate(error, 4_000);
        let result = sqlx::query(
            "UPDATE jobs SET status = $3, available_at = $4, last_error = $5, locked_at = NULL, \
             locked_by = NULL, updated_at = NOW() \
             WHERE id = $1 AND status = 'RUNNING' AND locked_by = $2",
        )
        .bind(job.id)
        .bind(worker_id)
        .bind(status.as_str())
        .bind(available_at)
        .bind(error)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 1 {
            Ok(status)
        } else {
            Err(CoreError::Conflict(
                "job lease is no longer owned by this worker".into(),
            ))
        }
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_bounded() {
        let policy = RetryPolicy {
            base_delay: Duration::seconds(2),
            max_delay: Duration::seconds(10),
        };
        assert_eq!(policy.delay_for_attempt(1), Duration::seconds(2));
        assert_eq!(policy.delay_for_attempt(3), Duration::seconds(8));
        assert_eq!(policy.delay_for_attempt(20), Duration::seconds(10));
    }
}
