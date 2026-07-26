use crate::error::CoreError;
use async_trait::async_trait;
use rig::agent::Agent;
use rig::completion::{AssistantContent, Completion, Message};
use rig::prelude::*;
use rig::providers::{anthropic, deepseek, gemini, openai};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug)]
pub struct LlmRequest {
    pub prompt: String,
    pub max_tokens: Option<u32>,
}

#[derive(Debug)]
pub struct LlmResponse {
    pub completion: String,
    pub usage: Option<LlmUsage>,
    pub provider: String,
    pub model: String,
    pub finish_reason: Option<String>,
    pub request_id: Option<String>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cached_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub tool_use_prompt_tokens: u64,
    pub reasoning_tokens: u64,
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
    model: &str,
) -> Result<LlmResponse, CoreError> {
    let started = Instant::now();
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

    let usage = response.usage;
    let request_id = response.message_id;

    Ok(LlmResponse {
        completion: text,
        usage: Some(LlmUsage {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            cached_input_tokens: usage.cached_input_tokens,
            cache_creation_input_tokens: usage.cache_creation_input_tokens,
            tool_use_prompt_tokens: usage.tool_use_prompt_tokens,
            reasoning_tokens: usage.reasoning_tokens,
        }),
        provider: provider.to_string(),
        model: model.to_string(),
        finish_reason: None,
        request_id,
        latency_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
    })
}

// ==========================================
// PROVIDER IMPLEMENTATIONS
// ==========================================
pub struct DeepSeekWrapper {
    agent: Agent<deepseek::CompletionModel>,
    model: String,
}

pub struct OpenAiWrapper {
    agent: Agent<openai::responses_api::ResponsesCompletionModel>,
    model: String,
}

pub struct GeminiWrapper {
    agent: Agent<gemini::completion::CompletionModel>,
    model: String,
}

pub struct AnthropicWrapper {
    agent: Agent<anthropic::completion::CompletionModel>,
    model: String,
}

#[async_trait]
impl Llm for DeepSeekWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        complete_with_agent(&self.agent, request, "DeepSeek", &self.model).await
    }
}

#[async_trait]
impl Llm for OpenAiWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        complete_with_agent(&self.agent, request, "OpenAI", &self.model).await
    }
}

#[async_trait]
impl Llm for GeminiWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        complete_with_agent(&self.agent, request, "Gemini", &self.model).await
    }
}

#[async_trait]
impl Llm for AnthropicWrapper {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        complete_with_agent(&self.agent, request, "Anthropic", &self.model).await
    }
}

// ==========================================
// FALLBACK / LOCAL DEV STUB
// ==========================================
pub struct StubClient;

#[async_trait]
impl Llm for StubClient {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError> {
        let _ = request;
        tracing::debug!("Stub LLM request received");
        Ok(LlmResponse {
            completion: format!("(Stubbed AI response for: {})", request.prompt),
            usage: None,
            provider: "Stub".to_string(),
            model: "stub".to_string(),
            finish_reason: Some("stop".to_string()),
            request_id: None,
            latency_ms: 0,
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
            let Ok(client) = deepseek::Client::from_env() else {
                tracing::warn!("DeepSeek credentials are unavailable; falling back to Stub");
                return Arc::new(StubClient);
            };
            let builder = client.agent(model);
            let agent = apply_max_tokens(builder, max_tokens).build();
            Arc::new(DeepSeekWrapper {
                agent,
                model: model.to_string(),
            })
        }
        ProviderKind::OpenAi => {
            tracing::info!("Initializing OpenAI LLM (model: {})", model);
            let Ok(client) = openai::Client::from_env() else {
                tracing::warn!("OpenAI credentials are unavailable; falling back to Stub");
                return Arc::new(StubClient);
            };
            let builder = client.agent(model);
            let agent = apply_max_tokens(builder, max_tokens).build();
            Arc::new(OpenAiWrapper {
                agent,
                model: model.to_string(),
            })
        }
        ProviderKind::Gemini => {
            tracing::info!("Initializing Gemini LLM (model: {})", model);
            let Ok(client) = gemini::Client::from_env() else {
                tracing::warn!("Gemini credentials are unavailable; falling back to Stub");
                return Arc::new(StubClient);
            };
            let builder = client.agent(model);
            let agent = apply_max_tokens(builder, max_tokens).build();
            Arc::new(GeminiWrapper {
                agent,
                model: model.to_string(),
            })
        }
        ProviderKind::Anthropic => {
            tracing::info!("Initializing Anthropic LLM (model: {})", model);
            let Ok(client) = anthropic::Client::from_env() else {
                tracing::warn!("Anthropic credentials are unavailable; falling back to Stub");
                return Arc::new(StubClient);
            };
            let builder = client.agent(model);
            let agent = apply_max_tokens(builder, max_tokens).build();
            Arc::new(AnthropicWrapper {
                agent,
                model: model.to_string(),
            })
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
