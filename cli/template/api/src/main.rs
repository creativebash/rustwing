mod domain;
mod error;
mod http;
mod openapi;
mod repository;
mod services;
mod state;

use rustwing::infrastructure::llm::{build_client_with_config, default_model_for_provider};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    if std::env::args().any(|arg| arg == "--openapi-json") {
        println!(
            "{}",
            openapi::openapi()
                .to_pretty_json()
                .expect("Failed to serialize OpenAPI document")
        );
        return;
    }

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
    let model = std::env::var("LLM_MODEL")
        .unwrap_or_else(|_| default_model_for_provider(&provider).to_string());
    let max_tokens = std::env::var("LLM_MAX_TOKENS")
        .ok()
        .and_then(|v| v.parse().ok());
    let llm = build_client_with_config(&provider, &model, max_tokens);

    let jwt_secret = std::env::var("JWT_SECRET").expect(
        "JWT_SECRET must be set. Generate a strong, unique secret; no insecure fallback is used.",
    );

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
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations. Applied migrations are immutable; restore any missing migration file instead of editing SQLx history.");
}
