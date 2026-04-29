//! `use rustwing::prelude::*;` for the most common framework imports.

pub use crate::error::CoreError;
pub use crate::infrastructure::auth::AuthEngine;
pub use crate::infrastructure::llm::{LlmRef, LlmRequest, LlmResponse};
pub use crate::repository::generic_crud;
pub use crate::repository::traits::*;
