use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Resource not found")]
    NotFound,

    #[error("Internal error: {0}")]
    Internal(String),
}
