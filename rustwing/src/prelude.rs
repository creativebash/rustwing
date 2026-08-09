//! `use rustwing::prelude::*;` for the most common framework imports.

pub use crate::error::CoreError;
pub use crate::infrastructure::auth::AuthEngine;
pub use crate::infrastructure::idempotency::{
    IdempotencyOptions, IdempotencyOutcome, IdempotencyRecord, IdempotencyScope, IdempotencyStore,
};
pub use crate::infrastructure::jobs::{ClaimedJob, JobOptions, JobQueue, JobStatus, RetryPolicy};
pub use crate::infrastructure::llm::{
    LlmRef, LlmRequest, LlmResponse, LlmUsage, build_client, build_client_with_config,
    default_model_for_provider,
};
pub use crate::infrastructure::outbox::{NewOutboxEvent, Outbox, OutboxEvent};
pub use crate::pagination::{CursorPage, decode_cursor, encode_cursor};
pub use crate::patch::Nullable;
pub use crate::repository::generic_crud;
pub use crate::repository::traits::*;
