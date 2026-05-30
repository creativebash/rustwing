use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use rustwing::prelude::*;
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;
use validator::ValidationErrors;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    Core(#[from] CoreError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationErrors),
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

fn error_response(code: &str, message: &str) -> Json<ErrorResponse> {
    Json(ErrorResponse {
        error: ErrorBody {
            code: code.to_string(),
            message: message.to_string(),
        },
    })
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
                                return (StatusCode::CONFLICT, error_response("conflict", "Resource already exists")).into_response();
                            }
                            "23503" => {
                                return (StatusCode::CONFLICT, error_response("conflict", "Referenced resource not found")).into_response();
                            }
                            _ => {}
                        }
                    }
                }
                tracing::error!("Database error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, error_response("internal_server_error", "Internal server error")).into_response()
            }
            AppError::Core(CoreError::NotFound) => {
                (StatusCode::NOT_FOUND, error_response("not_found", "Resource not found")).into_response()
            }
            AppError::Core(CoreError::Unauthorized) => {
                (StatusCode::UNAUTHORIZED, error_response("unauthorized", "You must be logged in to access this resource")).into_response()
            }
            AppError::Core(CoreError::Internal(msg)) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, error_response("internal_server_error", "Internal server error")).into_response()
            }
            AppError::Validation(err) => {
                tracing::warn!("Validation error: {:?}", err);
                (StatusCode::BAD_REQUEST, error_response("validation_error", &err.to_string())).into_response()
            }
        }
    }
}
