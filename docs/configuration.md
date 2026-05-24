# Configuration

All configuration is via environment variables. Copy `.env.example` to `.env` and edit.

## Database

| Variable | Required | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | Yes | — | Postgres connection string |

Format: `postgres://<user>:<password>@<host>:<port>/<database>`

## Auth

| Variable | Required | Default | Description |
|---|---|---|---|
| `JWT_SECRET` | No | dev-only fallback | Secret key for signing JWT tokens |

In production, always set a strong, unique `JWT_SECRET`.

## LLM

| Variable | Required | Default | Description |
|---|---|---|---|
| `LLM_PROVIDER` | No | `stub` | Provider: `"stub"`, `"deepseek"`, `"openai"`, `"gemini"`, or `"anthropic"` |
| `LLM_MODEL` | No | provider default | Model name for the selected provider |
| `DEEPSEEK_API_KEY` | For DeepSeek | — | API key from DeepSeek |
| `OPENAI_API_KEY` | For OpenAI | — | API key from OpenAI |
| `GEMINI_API_KEY` | For Gemini | — | API key from Google Gemini |
| `ANTHROPIC_API_KEY` | For Anthropic | — | API key from Anthropic |

Set `LLM_PROVIDER=stub` for local development — no API key needed. The stub logs prompts and returns canned responses. If `LLM_MODEL` is unset, Rustwing picks a provider-specific default: `deepseek-chat`, `gpt-4o`, `gemini-2.5-flash`, or `claude-sonnet-4-6`.

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

The worker uses the same `DATABASE_URL`, `LLM_PROVIDER`, `LLM_MODEL`, and provider API keys as the API binary.
