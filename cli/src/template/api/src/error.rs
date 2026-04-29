use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rustwing::prelude::*;
use serde_json::json;
use thiserror::Error;
use validator::ValidationErrors;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    Core(#[from] CoreError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationErrors),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Core(CoreError::Database(err))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Core(CoreError::Database(ref err)) => {
                if let Some(db_err) = err.as_database_error() {
                    if let Some(code) = db_err.code() {
                        match code.as_ref() {
                            "23505" => {
                                return (StatusCode::CONFLICT, Json(json!({ "error": "Resource already exists" }))).into_response();
                            }
                            "23503" => {
                                return (StatusCode::CONFLICT, Json(json!({ "error": "Referenced resource not found" }))).into_response();
                            }
                            _ => {}
                        }
                    }
                }
                tracing::error!("Database error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Internal server error" }))).into_response()
            }
            AppError::Core(CoreError::NotFound) => {
                (StatusCode::NOT_FOUND, Json(json!({ "error": "Resource not found" }))).into_response()
            }
            AppError::Core(CoreError::Internal(msg)) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Internal server error" }))).into_response()
            }
            AppError::Validation(err) => {
                tracing::warn!("Validation error: {:?}", err);
                (StatusCode::BAD_REQUEST, Json(json!({ "error": err.to_string() }))).into_response()
            }
        }
    }
}
