mod domain;
mod error;
mod http;
mod repository;
mod services;
mod state;

use rustwing::infrastructure::llm::build_client;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // 1. Load environment variables
    dotenvy::dotenv().ok();

    // 2. Setup standard observability/logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,rust_api=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Server...");

    // 3. Setup robust DB Connection Pool
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&db_url)
        .await
        .expect("Failed to connect to Postgres");

    // 4. Run database migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // 5. Construct Application State
    let provider = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "stub".to_string());
    let model = std::env::var("LLM_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string());

    let llm = build_client(&provider, &model);

    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super_secret_dev_key_change_me".to_string());

    // ✅ Fix: Instantiate AppState ONCE with all required fields
    let state = state::AppState {
        db: pool,
        llm,
        jwt_secret,
    };

    // 5. Mount modular routes
    let app = http::app_router(state);

    // 6. Start Server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}
