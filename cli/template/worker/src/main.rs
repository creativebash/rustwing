use rustwing::infrastructure::llm::build_client;
use rustwing::prelude::*;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct WorkerState {
    db: PgPool,
    llm: LlmRef,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,worker=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Rustwing Worker starting...");

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await
        .expect("Failed to connect to Postgres");

    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "stub".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string());
    let llm = build_client(&provider, &model);

    let state = WorkerState { db: pool, llm };
    let tick_seconds = std::env::var("WORKER_TICK_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(10);
    let mut interval = tokio::time::interval(Duration::from_secs(tick_seconds));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(err) = process_pending_jobs(&state).await {
                    tracing::error!("Worker tick failed: {:?}", err);
                }
            }
            signal = tokio::signal::ctrl_c() => {
                if let Err(err) = signal {
                    tracing::error!("Failed to listen for shutdown signal: {}", err);
                }
                break;
            }
        }
    }

    tracing::info!("Worker shutting down.");
}

async fn process_pending_jobs(state: &WorkerState) -> Result<(), CoreError> {
    let _db = &state.db;
    let _llm = &state.llm;

    tracing::debug!("Worker tick complete; no jobs are registered yet");
    Ok(())
}
