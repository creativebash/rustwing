mod config;
mod domain;
mod error;
mod http;
mod openapi;
mod repository;
mod services;
mod state;

use config::{AppConfig, Environment};
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

    let config =
        AppConfig::from_env().unwrap_or_else(|error| panic!("Invalid configuration: {error}"));
    let filter = tracing_subscriber::EnvFilter::new(
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info,api=debug".into()),
    );
    if config.environment == Environment::Production {
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

    tracing::info!("Starting Rustwing API...");

    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to Postgres");

    run_migrations(&pool).await;

    let model = if config.llm_model.is_empty() {
        default_model_for_provider(&config.llm_provider)
    } else {
        &config.llm_model
    };
    let llm = build_client_with_config(&config.llm_provider, model, config.llm_max_tokens)
        .expect("LLM configuration was validated but client initialization failed");

    let state = state::AppState {
        db: pool,
        llm,
        jwt_secret: config.jwt_secret,
        rate_limit: config.rate_limit,
    };

    let port = config.port;
    let addr = format!("0.0.0.0:{port}");
    let app = http::app_router(state);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on http://localhost:{port}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
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
    tracing::info!("shutdown requested");
}

async fn run_migrations(pool: &sqlx::PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations. Applied migrations are immutable; restore any missing migration file instead of editing SQLx history.");
}
