//! `use rustwing::prelude::*;` for the most common framework imports.

pub use crate::error::CoreError;
pub use crate::infrastructure::auth::AuthEngine;
pub use crate::infrastructure::llm::{
    LlmRef, LlmRequest, LlmResponse, build_client, build_client_with_config,
    default_model_for_provider,
};
pub use crate::patch::Nullable;
pub use crate::repository::generic_crud;
pub use crate::repository::traits::*;
