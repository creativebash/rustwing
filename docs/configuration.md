# Configuration

All configuration is via environment variables. Copy `.env.example` to `.env` and edit.

## Environment

| Variable | Required | Default | Description |
|---|---|---|---|
| `APP_ENV` | No | `development` | `development`, `test`, or `production` |

Production mode emits JSON logs and rejects development-only or incomplete
security configuration. Development defaults are never silently promoted to
production behavior.

## Database

| Variable | Required | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | Yes | — | Postgres connection string |

Format: `postgres://<user>:<password>@<host>:<port>/<database>`

## Auth

| Variable | Required | Default | Description |
|---|---|---|---|
| `JWT_SECRET` | Yes | — | Secret key for signing JWT tokens |

Rustwing does not provide a fallback JWT secret. Generate a strong, unique
secret for every environment; the API fails at startup when it is absent.
Production also rejects secrets shorter than 32 characters and known example
placeholders. The value is not logged.

## LLM

| Variable | Required | Default | Description |
|---|---|---|---|
| `LLM_PROVIDER` | No | `stub` | Provider: `"disabled"`, `"stub"`, `"deepseek"`, `"openai"`, `"gemini"`, or `"anthropic"` |
| `LLM_MODEL` | No | provider default | Model name for the selected provider |
| `DEEPSEEK_API_KEY` | For DeepSeek | — | API key from DeepSeek |
| `OPENAI_API_KEY` | For OpenAI | — | API key from OpenAI |
| `GEMINI_API_KEY` | For Gemini | — | API key from Google Gemini |
| `ANTHROPIC_API_KEY` | For Anthropic | — | API key from Anthropic |
| `LLM_MAX_TOKENS` | No | — | Default max output tokens (safety cap). Override per-request in code. |

Set `LLM_PROVIDER=stub` for local development — no API key needed. The stub
returns a canned response without usage metrics and does not log prompt
contents. If `LLM_MODEL` is unset, Rustwing picks a provider-specific default:
`deepseek-chat`, `gpt-4o`, `gemini-2.5-flash`, or `claude-sonnet-4-6`.

Per-request `max_tokens` override is supported in service code via `LlmRequest { prompt, max_tokens: Some(512) }`. When set, it takes precedence over the `LLM_MAX_TOKENS` env default.

The stub is rejected in production; use `disabled` when a production binary
does not need LLM calls. Unknown providers, missing credentials,
and provider initialization failures stop startup instead of falling back.

## Logging

| Variable | Required | Default | Description |
|---|---|---|---|
| `RUST_LOG` | No | `info,api=debug` | Log level and targets |

Format: `[target=]level[,target=level...]`

Examples:
- `info` — info and above for all crates
- `info,api=debug` — debug for your app, info for dependencies
- `trace` — everything (very verbose)

## Worker

| Variable | Required | Default | Description |
|---|---|---|---|
| `WORKER_TICK_SECONDS` | No | `10` | Seconds between worker processing ticks |
| `WORKER_LEASE_SECONDS` | No | `60` | Job lease duration; active jobs heartbeat at one third of this interval |
| `WORKER_BATCH_SIZE` | No | `10` | Jobs claimed per polling batch, capped at 100 |
| `WORKER_ID` | No | Generated UUIDv7 name | Stable diagnostic identity for this process |

The worker uses the same `DATABASE_URL`, `LLM_PROVIDER`, `LLM_MODEL`, and provider API keys as the API binary.

## Rate limiting

| Variable | Required | Default | Description |
|---|---|---|---|
| `RATE_LIMIT_GLOBAL_REQUESTS` | No | `600` | Requests per process-local window |
| `RATE_LIMIT_AUTH_REQUESTS` | No | `10` | Login/registration requests per process-local window |
| `RATE_LIMIT_WINDOW_SECONDS` | No | `60` | Window duration |

The limiter is per process and is not a globally distributed rate limiter.
It keys requests by the direct peer IP. Deployments behind a proxy or with
multiple replicas should enforce the authoritative client policy at a trusted
edge rather than trusting arbitrary forwarded headers.
