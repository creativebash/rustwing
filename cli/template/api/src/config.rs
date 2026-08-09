use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

impl Environment {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "production" | "prod" => Ok(Self::Production),
            _ => Err("APP_ENV must be development, test, or production".into()),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitConfig {
    pub global_requests: u64,
    pub auth_requests: u64,
    pub window: Duration,
}

#[derive(Clone)]
pub struct AppConfig {
    pub environment: Environment,
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub llm_provider: String,
    pub llm_model: String,
    pub llm_max_tokens: Option<u32>,
    pub rate_limit: RateLimitConfig,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        Self::from_getter(|name| std::env::var(name).ok())
    }

    fn from_getter(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let environment = Environment::parse(get("APP_ENV").as_deref().unwrap_or("development"))?;
        let database_url = required(&get, "DATABASE_URL")?;
        let jwt_secret = required(&get, "JWT_SECRET")?;
        if environment == Environment::Production
            && (jwt_secret.len() < 32
                || jwt_secret.contains("replace_with")
                || jwt_secret.contains("change_me"))
        {
            return Err("JWT_SECRET must be a strong, unique production secret of at least 32 characters".into());
        }
        let llm_provider = get("LLM_PROVIDER")
            .unwrap_or_else(|| "stub".into())
            .to_ascii_lowercase();
        if environment == Environment::Production && llm_provider == "stub" {
            return Err("LLM_PROVIDER=stub is development-only".into());
        }
        let credential = match llm_provider.as_str() {
            "stub" | "disabled" | "none" => None,
            "deepseek" => Some("DEEPSEEK_API_KEY"),
            "openai" => Some("OPENAI_API_KEY"),
            "gemini" | "google" => Some("GEMINI_API_KEY"),
            "anthropic" | "claude" => Some("ANTHROPIC_API_KEY"),
            _ => return Err(format!("unknown LLM provider: {llm_provider}")),
        };
        if let Some(name) = credential {
            required(&get, name)?;
        }
        let llm_model = get("LLM_MODEL").unwrap_or_default();
        Ok(Self {
            environment,
            database_url,
            jwt_secret,
            port: parse(&get, "PORT", 3000)?,
            llm_provider,
            llm_model,
            llm_max_tokens: optional_parse(&get, "LLM_MAX_TOKENS")?,
            rate_limit: RateLimitConfig {
                global_requests: parse(&get, "RATE_LIMIT_GLOBAL_REQUESTS", 600)?,
                auth_requests: parse(&get, "RATE_LIMIT_AUTH_REQUESTS", 10)?,
                window: Duration::from_secs(parse(&get, "RATE_LIMIT_WINDOW_SECONDS", 60)?),
            },
        })
    }
}

fn required(get: &impl Fn(&str) -> Option<String>, name: &str) -> Result<String, String> {
    get(name)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{name} must be set"))
}

fn parse<T: std::str::FromStr>(
    get: &impl Fn(&str) -> Option<String>,
    name: &str,
    default: T,
) -> Result<T, String> {
    match get(name) {
        Some(value) => value
            .parse()
            .map_err(|_| format!("{name} has an invalid value")),
        None => Ok(default),
    }
}

fn optional_parse<T: std::str::FromStr>(
    get: &impl Fn(&str) -> Option<String>,
    name: &str,
) -> Result<Option<T>, String> {
    get(name)
        .map(|value| {
            value
                .parse()
                .map_err(|_| format!("{name} has an invalid value"))
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(values: &[(&str, &str)]) -> Result<AppConfig, String> {
        let values: HashMap<_, _> = values
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        AppConfig::from_getter(|name| values.get(name).cloned())
    }

    #[test]
    fn production_rejects_unsafe_secrets_and_stub() {
        assert!(
            config(&[
                ("APP_ENV", "production"),
                ("DATABASE_URL", "postgres://db"),
                ("JWT_SECRET", "change_me")
            ])
            .is_err()
        );
        assert!(
            config(&[
                ("APP_ENV", "production"),
                ("DATABASE_URL", "postgres://db"),
                ("JWT_SECRET", "01234567890123456789012345678901"),
                ("LLM_PROVIDER", "stub")
            ])
            .is_err()
        );
    }

    #[test]
    fn development_allows_explicit_local_defaults() {
        assert!(
            config(&[
                ("DATABASE_URL", "postgres://db"),
                ("JWT_SECRET", "local-only")
            ])
            .is_ok()
        );
    }

    #[test]
    fn production_can_explicitly_disable_llm() {
        assert!(
            config(&[
                ("APP_ENV", "production"),
                ("DATABASE_URL", "postgres://db"),
                ("JWT_SECRET", "01234567890123456789012345678901"),
                ("LLM_PROVIDER", "disabled")
            ])
            .is_ok()
        );
    }
}
