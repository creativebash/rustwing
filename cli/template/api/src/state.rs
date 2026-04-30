use rustwing::prelude::*;
use sqlx::PgPool;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub db: PgPool,
    pub llm: LlmRef,
    pub jwt_secret: String,
}
