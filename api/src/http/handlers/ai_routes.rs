use crate::{
    error::AppError,
    http::dtos::ai_dto::{AiRequest, AiResponse},
    http::extractors::AuthUser,
    state::AppState,
};
use axum::{Json, extract::State};
use rustwing::infrastructure::llm::LlmRequest;

pub async fn complete(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<AiRequest>,
) -> Result<Json<AiResponse>, AppError> {
    let response = state
        .llm
        .complete(LlmRequest {
            prompt: req.prompt,
            max_tokens: req.max_tokens,
        })
        .await?;
    Ok(Json(AiResponse {
        completion: response.completion,
    }))
}
