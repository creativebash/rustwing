use rustwing::prelude::*;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub llm: LlmRef,
    pub jwt_secret: String,
}
