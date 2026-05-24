use crate::error::CoreError;
use async_trait::async_trait;
use rig::agent::Agent;
use rig::completion::{AssistantContent, Completion, Message};
use rig::prelude::*;
use rig::providers::{anthropic, deepseek, gemini, openai};
use std::sync::Arc;

#[derive(Debug)]
pub struct LlmRequest {
    pub prompt: String,
    pub max_tokens: Option<u32>,
}

#[derive(Debug)]
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

async fn complete_with_agent<M: rig::completion::CompletionModel>(
    agent: &Agent<M>,
    request: LlmRequest,
    provider: &str,
) -> Result<LlmResponse, CoreError> {
    let mut builder = agent
        .completion(&request.prompt, Vec::<Message>::new())
        .await
        .map_err(|e| CoreError::Internal(format!("{} completion error: {}", provider, e)))?;

    if let Some(t) = request.max_tokens {
        builder = builder.max_tokens_opt(Some(t as u64));
    }

    let response = builder
        .send()
        .await
        .map_err(|e| CoreError::Internal(format!("{} send error: {}", provider, e)))?;

    let text = response
        .choice
        .into_iter()
        .filter_map(|content| {
            if let AssistantContent::Text(t) = content {
                Some(t.text)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(LlmResponse { completion: text })
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
        complete_with_agent(&self.agent, request, "DeepSeek").await
    }
}

#[async_trait]
impl Llm for OpenAiWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        complete_with_agent(&self.agent, request, "OpenAI").await
    }
}

#[async_trait]
impl Llm for GeminiWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        complete_with_agent(&self.agent, request, "Gemini").await
    }
}

#[async_trait]
impl Llm for AnthropicWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        complete_with_agent(&self.agent, request, "Anthropic").await
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
fn apply_max_tokens<M: rig::completion::CompletionModel>(
    builder: rig::agent::AgentBuilder<M>,
    max_tokens: Option<u64>,
) -> rig::agent::AgentBuilder<M> {
    if let Some(t) = max_tokens {
        builder.max_tokens(t)
    } else {
        builder
    }
}

pub fn build_client(provider: &str, model: &str) -> LlmRef {
    build_client_with_config(provider, model, None)
}

pub fn build_client_with_config(provider: &str, model: &str, max_tokens: Option<u32>) -> LlmRef {
    let provider_kind = provider_kind(provider);
    let model = model.trim();
    let model = if model.is_empty() {
        default_model_for_provider(provider)
    } else {
        model
    };

    let max_tokens = max_tokens.map(|t| t as u64);

    match provider_kind {
        ProviderKind::Stub => {
            tracing::info!("Initializing Stub LLM");
            Arc::new(StubClient)
        }
        ProviderKind::DeepSeek => {
            tracing::info!("Initializing DeepSeek LLM (model: {})", model);
            let builder = deepseek::Client::from_env().agent(model);
            let agent = apply_max_tokens(builder, max_tokens).build();
            Arc::new(DeepSeekWrapper { agent })
        }
        ProviderKind::OpenAi => {
            tracing::info!("Initializing OpenAI LLM (model: {})", model);
            let builder = openai::Client::from_env().agent(model);
            let agent = apply_max_tokens(builder, max_tokens).build();
            Arc::new(OpenAiWrapper { agent })
        }
        ProviderKind::Gemini => {
            tracing::info!("Initializing Gemini LLM (model: {})", model);
            let builder = gemini::Client::from_env().agent(model);
            let agent = apply_max_tokens(builder, max_tokens).build();
            Arc::new(GeminiWrapper { agent })
        }
        ProviderKind::Anthropic => {
            tracing::info!("Initializing Anthropic LLM (model: {})", model);
            let builder = anthropic::Client::from_env().agent(model);
            let agent = apply_max_tokens(builder, max_tokens).build();
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
