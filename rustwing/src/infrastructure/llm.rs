use crate::error::CoreError;
use async_trait::async_trait;
use rig::agent::Agent;
use rig::completion::Prompt;
use rig::prelude::*;
use rig::providers::{anthropic, deepseek, gemini, openai};
use std::sync::Arc;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderKind {
    Stub,
    DeepSeek,
    OpenAi,
    Gemini,
    Anthropic,
    Unknown,
}

fn provider_kind(provider: &str) -> ProviderKind {
    match provider.trim().to_ascii_lowercase().as_str() {
        "stub" => ProviderKind::Stub,
        "deepseek" => ProviderKind::DeepSeek,
        "openai" => ProviderKind::OpenAi,
        "gemini" | "google" => ProviderKind::Gemini,
        "anthropic" | "claude" => ProviderKind::Anthropic,
        _ => ProviderKind::Unknown,
    }
}

pub fn default_model_for_provider(provider: &str) -> &'static str {
    match provider_kind(provider) {
        ProviderKind::DeepSeek => "deepseek-chat",
        ProviderKind::OpenAi => openai::GPT_4O,
        ProviderKind::Gemini => gemini::completion::GEMINI_2_5_FLASH,
        ProviderKind::Anthropic => anthropic::completion::CLAUDE_SONNET_4_6,
        ProviderKind::Stub | ProviderKind::Unknown => "stub",
    }
}

// ==========================================
// PROVIDER IMPLEMENTATIONS
// ==========================================
pub struct DeepSeekWrapper {
    agent: Agent<deepseek::CompletionModel>,
}

pub struct OpenAiWrapper {
    agent: Agent<openai::responses_api::ResponsesCompletionModel>,
}

pub struct GeminiWrapper {
    agent: Agent<gemini::completion::CompletionModel>,
}

pub struct AnthropicWrapper {
    agent: Agent<anthropic::completion::CompletionModel>,
}

#[async_trait]
impl Llm for DeepSeekWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        let response = self
            .agent
            .prompt(&request.prompt)
            .await
            .map_err(|e| CoreError::Internal(format!("DeepSeek error: {}", e)))?;
        Ok(LlmResponse {
            completion: response,
        })
    }
}

#[async_trait]
impl Llm for OpenAiWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        let response = self
            .agent
            .prompt(&request.prompt)
            .await
            .map_err(|e| CoreError::Internal(format!("OpenAI error: {}", e)))?;
        Ok(LlmResponse {
            completion: response,
        })
    }
}

#[async_trait]
impl Llm for GeminiWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        let response = self
            .agent
            .prompt(&request.prompt)
            .await
            .map_err(|e| CoreError::Internal(format!("Gemini error: {}", e)))?;
        Ok(LlmResponse {
            completion: response,
        })
    }
}

#[async_trait]
impl Llm for AnthropicWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        let response = self
            .agent
            .prompt(&request.prompt)
            .await
            .map_err(|e| CoreError::Internal(format!("Anthropic error: {}", e)))?;
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
    let provider_kind = provider_kind(provider);
    let model = model.trim();
    let model = if model.is_empty() {
        default_model_for_provider(provider)
    } else {
        model
    };

    match provider_kind {
        ProviderKind::Stub => {
            tracing::info!("Initializing Stub LLM");
            Arc::new(StubClient)
        }
        ProviderKind::DeepSeek => {
            tracing::info!("Initializing DeepSeek LLM (model: {})", model);
            let client = deepseek::Client::from_env();
            let agent = client.agent(model).build();
            Arc::new(DeepSeekWrapper { agent })
        }
        ProviderKind::OpenAi => {
            tracing::info!("Initializing OpenAI LLM (model: {})", model);
            let client = openai::Client::from_env();
            let agent = client.agent(model).build();
            Arc::new(OpenAiWrapper { agent })
        }
        ProviderKind::Gemini => {
            tracing::info!("Initializing Gemini LLM (model: {})", model);
            let client = gemini::Client::from_env();
            let agent = client.agent(model).build();
            Arc::new(GeminiWrapper { agent })
        }
        ProviderKind::Anthropic => {
            tracing::info!("Initializing Anthropic LLM (model: {})", model);
            let client = anthropic::Client::from_env();
            let agent = client.agent(model).build();
            Arc::new(AnthropicWrapper { agent })
        }
        ProviderKind::Unknown => {
            tracing::warn!("Unknown provider '{}'. Falling back to Stub.", provider);
            Arc::new(StubClient)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_provider_aliases() {
        assert_eq!(provider_kind("openai"), ProviderKind::OpenAi);
        assert_eq!(provider_kind("google"), ProviderKind::Gemini);
        assert_eq!(provider_kind("claude"), ProviderKind::Anthropic);
        assert_eq!(provider_kind("unknown"), ProviderKind::Unknown);
    }

    #[test]
    fn returns_provider_specific_default_models() {
        assert_eq!(default_model_for_provider("stub"), "stub");
        assert_eq!(default_model_for_provider("deepseek"), "deepseek-chat");
        assert_eq!(default_model_for_provider("openai"), openai::GPT_4O);
        assert_eq!(
            default_model_for_provider("gemini"),
            gemini::completion::GEMINI_2_5_FLASH
        );
        assert_eq!(
            default_model_for_provider("anthropic"),
            anthropic::completion::CLAUDE_SONNET_4_6
        );
    }
}
