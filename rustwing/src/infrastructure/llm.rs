use crate::error::CoreError;
use async_trait::async_trait;
use std::sync::Arc;

use rig::completion::Prompt;
use rig::prelude::*;
use rig::providers::deepseek;

pub struct LlmRequest {
    pub prompt: String,
    pub max_tokens: Option<u32>,
}

pub struct LlmResponse {
    pub completion: String,
}

#[async_trait]
pub trait Llm: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError>;
}

pub type LlmRef = Arc<dyn Llm>;

// ==========================================
// DEEPSEEK IMPLEMENTATION
// ==========================================
pub struct DeepSeekWrapper {
    agent: rig::agent::Agent<deepseek::CompletionModel>,
}

#[async_trait]
impl Llm for DeepSeekWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        let response: String = self
            .agent
            .prompt(&request.prompt)
            .await
            .map_err(|e| CoreError::Internal(format!("DeepSeek Error: {}", e)))?;
        Ok(LlmResponse {
            completion: response,
        })
    }
}

// ==========================================
// FALLBACK / LOCAL DEV STUB
// ==========================================
pub struct StubClient;

#[async_trait]
impl Llm for StubClient {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        tracing::info!("Stub LLM received prompt: {}", request.prompt);
        Ok(LlmResponse {
            completion: format!("(Stubbed AI response for: {})", request.prompt),
        })
    }
}

// ==========================================
// FACTORY
// ==========================================
pub fn build_client(provider: &str, model: &str) -> LlmRef {
    match provider.to_lowercase().as_str() {
        "deepseek" => {
            tracing::info!("Initializing DeepSeek LLM (Model: {})", model);
            // from_env() reads DEEPSEEK_API_KEY automatically
            let client = deepseek::Client::from_env();
            // model is your configurable LLM_MODEL from .env e.g. "deepseek-chat"
            let agent = client.agent(model).build();
            Arc::new(DeepSeekWrapper { agent })
        }
        _ => {
            tracing::warn!("Unknown provider '{}'. Falling back to Stub.", provider);
            Arc::new(StubClient)
        }
    }
}

// how to use in main.rs or wherever:
/*
let llm_client = build_client(&config.llm_provider, &config.llm_model);
let response = llm_client.complete(LlmRequest {
    prompt: "What is the capital of France?".to_string(),
    max_tokens: Some(50),
}).await?;
println!("LLM Response: {}", response.completion);
*/
