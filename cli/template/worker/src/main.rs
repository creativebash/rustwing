use chrono::Duration as ChronoDuration;
use rustwing::infrastructure::llm::{build_client_with_config, default_model_for_provider};
use rustwing::prelude::*;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct WorkerState {
    queue: JobQueue,
    db: PgPool,
    llm: LlmRef,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let environment = std::env::var("APP_ENV")
        .unwrap_or_else(|_| "development".into())
        .to_ascii_lowercase();
    let production = match environment.as_str() {
        "production" | "prod" => true,
        "development" | "dev" | "test" => false,
        _ => panic!("APP_ENV must be development, test, or production"),
    };
    let filter = tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info,worker=debug".into()),
    );
    if production {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    let db_url = required("DATABASE_URL");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await
        .expect("Failed to connect to Postgres");
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "stub".into());
    if production && provider.eq_ignore_ascii_case("stub") {
        panic!("LLM_PROVIDER=stub is development-only");
    }
    let model =
        std::env::var("LLM_MODEL").unwrap_or_else(|_| default_model_for_provider(&provider).into());
    let max_tokens = std::env::var("LLM_MAX_TOKENS")
        .ok()
        .map(|value| value.parse().expect("LLM_MAX_TOKENS must be an integer"));
    let llm =
        build_client_with_config(&provider, &model, max_tokens).expect("Invalid LLM configuration");
    let state = WorkerState {
        queue: JobQueue::new(pool.clone()),
        db: pool,
        llm,
    };
    run(state).await;
}

async fn run(state: WorkerState) {
    let stopping = Arc::new(AtomicBool::new(false));
    let signal_flag = stopping.clone();
    tokio::spawn(async move {
        shutdown_signal().await;
        signal_flag.store(true, Ordering::Release);
    });
    let worker_id =
        std::env::var("WORKER_ID").unwrap_or_else(|_| format!("worker-{}", uuid::Uuid::now_v7()));
    let tick = parse_env("WORKER_TICK_SECONDS", 10_u64);
    let lease = ChronoDuration::seconds(parse_env("WORKER_LEASE_SECONDS", 60_i64).max(5));
    let batch = parse_env("WORKER_BATCH_SIZE", 10_i64).clamp(1, 100);
    let retry_policy = RetryPolicy::default();

    while !stopping.load(Ordering::Acquire) {
        match state.queue.claim(&worker_id, batch, lease).await {
            Ok(jobs) if jobs.is_empty() => tokio::time::sleep(Duration::from_secs(tick)).await,
            Ok(jobs) => {
                // Heartbeat every claimed job immediately. A job waiting behind a
                // long-running sibling must not become claimable in another process.
                let mut heartbeats: HashMap<_, _> = jobs
                    .iter()
                    .map(|job| {
                        (
                            job.id,
                            start_heartbeat(state.queue.clone(), worker_id.clone(), job.id, lease),
                        )
                    })
                    .collect();
                for job in jobs {
                    let span = tracing::info_span!("job", job_id=%job.id, job_type=%job.job_type, correlation_id=job.correlation_id.as_deref().unwrap_or(""), organisation_id=?job.organisation_id);
                    let _entered = span.enter();
                    let result = handle_job(&state, &job).await;
                    if let Some(heartbeat) = heartbeats.remove(&job.id) {
                        heartbeat.abort();
                    }
                    match result {
                        Ok(()) => {
                            if let Err(error) = state.queue.complete(&worker_id, job.id).await {
                                tracing::error!(%error, "job completion failed");
                            }
                        }
                        Err(failure) => {
                            if let Err(error) = state
                                .queue
                                .fail(
                                    &worker_id,
                                    &job,
                                    &failure.message,
                                    failure.retryable,
                                    &retry_policy,
                                )
                                .await
                            {
                                tracing::error!(%error, "job failure transition failed");
                            }
                        }
                    }
                }
            }
            Err(error) => {
                tracing::error!(%error, "job claim failed");
                tokio::time::sleep(Duration::from_secs(tick)).await;
            }
        }
    }
    tracing::info!("worker stopped after in-flight work completed");
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

fn start_heartbeat(
    queue: JobQueue,
    worker_id: String,
    job_id: uuid::Uuid,
    lease: ChronoDuration,
) -> tokio::task::JoinHandle<()> {
    let interval = Duration::from_secs((lease.num_seconds().max(6) / 3) as u64);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            match queue.heartbeat(&worker_id, job_id).await {
                Ok(true) => tracing::debug!(%job_id, "job lease renewed"),
                Ok(false) => break,
                Err(error) => tracing::error!(%job_id, %error, "job heartbeat failed"),
            }
        }
    })
}

struct JobFailure {
    message: String,
    retryable: bool,
}

async fn handle_job(state: &WorkerState, job: &ClaimedJob) -> Result<(), JobFailure> {
    let _ = (&state.db, &state.llm);
    // Application job types and `job.deserialize_payload::<T>()` belong here.
    // Unknown or malformed payloads are permanent failures and are never logged in full.
    Err(JobFailure {
        message: format!("unknown job type: {}", job.job_type),
        retryable: false,
    })
}

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set"))
}
fn parse_env<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .map(|value| {
            value
                .parse()
                .unwrap_or_else(|_| panic!("{name} has an invalid value"))
        })
        .unwrap_or(default)
}
