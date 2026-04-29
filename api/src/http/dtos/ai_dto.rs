// src/api/dtos/ai_dto.rs
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AiRequest {
    pub prompt: String,
    pub max_tokens: Option<u32>,
}

#[derive(Serialize)]
pub struct AiResponse {
    pub completion: String,
}
