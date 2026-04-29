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
| `LLM_PROVIDER` | No | `stub` | Provider: `"stub"` or `"deepseek"` |
| `LLM_MODEL` | No | `deepseek-chat` | Model name for the provider |
| `DEEPSEEK_API_KEY` | For DeepSeek | — | API key from DeepSeek |
| `OPENAI_API_KEY` | For OpenAI | — | API key from OpenAI |

Set `LLM_PROVIDER=stub` for local development — no API key needed. The stub logs prompts and returns canned responses.

## Logging

| Variable | Required | Default | Description |
|---|---|---|---|
| `RUST_LOG` | No | `info,api=debug` | Log level and targets |

Format: `[target=]level[,target=level...]`

Examples:
- `info` — info and above for all crates
- `info,api=debug` — debug for your app, info for dependencies
- `trace` — everything (very verbose)
