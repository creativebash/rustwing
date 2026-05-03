mod domain;
mod error;
mod http;
mod repository;
mod services;
mod state;

use rustwing::infrastructure::llm::build_client;
use sqlx::migrate::MigrateError;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,api=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Rustwing API...");

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&db_url)
        .await
        .expect("Failed to connect to Postgres");

    run_migrations(&pool).await;

    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "stub".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string());
    let llm = build_client(&provider, &model);

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super_secret_dev_key_change_me".to_string());

    let state = state::AppState {
        db: pool,
        llm,
        jwt_secret,
    };

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    let app = http::app_router(state);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on http://localhost:{port}");
    axum::serve(listener, app).await.unwrap();
}

async fn run_migrations(pool: &sqlx::PgPool) {
    match sqlx::migrate!("./migrations").run(pool).await {
        Ok(_) => {}
        Err(MigrateError::VersionMissing(version)) => {
            tracing::warn!(
                "Migration version {} was applied but file not found. Removing stale tracking entry.",
                version
            );
            sqlx::query("DELETE FROM _sqlx_migrations WHERE version = $1")
                .bind(version as i64)
                .execute(pool)
                .await
                .expect("Failed to clean stale migration entry");
            sqlx::migrate!("./migrations")
                .run(pool)
                .await
                .expect("Failed to run migrations after cleanup");
        }
        Err(e) => {
            panic!("Failed to run migrations: {}", e);
        }
    }
}
