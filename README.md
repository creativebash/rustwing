<div align="center">

# Rustwing

**The application framework for building Rust apps with Axum.**

Rustwing is a batteries-included framework for developers who want to build production-ready Rust backends and APIs quickly—especially SaaS and AI-enabled applications. Built on top of Axum and SQLx, it provides strong conventions and structure so you can focus on your product instead of boilerplate. Its explicit patterns also make your codebase easy for coding agents to understand and extend.

[![Crates.io](https://img.shields.io/crates/v/rustwing.svg)](https://crates.io/crates/rustwing)
[![Crates.io](https://img.shields.io/crates/v/rustwing-cli.svg)](https://crates.io/crates/rustwing-cli)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</div>

## What you get

- **Auth** — service-first Argon2 + JWT authentication with self-only starter account routes
- **Service-first CRUD scaffolding** — Generate REST endpoints, services, repositories, and migrations
- **Scoped resources** — Opt into SaaS-style or parent-child routes and SQLx helpers with `--tenant` or `--scope`
- **API-first by default** — OpenAPI JSON, Swagger UI, ReDoc, and TypeScript client generation for generated APIs
- **Migrations** — Automatic, fail-closed database migrations on run
- **Reliable background work** — PostgreSQL jobs, leases, retries, dead letters, transactional outbox, and idempotency
- **Production HTTP defaults** — request IDs, structured tracing, rate limits, graceful shutdown, liveness, and readiness
- **LLM hooks** — Pluggable AI integrations (DeepSeek, OpenAI, Gemini, Anthropic, local stubs)
- **Scaffolding CLI** — Generate resources, models, services, repositories, handlers, and routes instantly
- **Error handling** — Clean mapping of database and application errors

## Quick start

```bash
cargo install rustwing-cli
rustwing new my_app
cd my_app
rustwing doctor
rustwing upgrade             # Preview a safe framework upgrade
rustwing run
# Open http://localhost:3000/docs
```

## Philosophy

Rustwing is not a replacement for Axum — it builds on top of it.

- **Axum handles HTTP**
- **Rustwing handles your application**
- **SQLx keeps SQL explicit**

It provides a structured, batteries-included starting point for building real-world Rust apps, especially SaaS-style backends.

## Positioning

Rustwing is an **application framework**, not a low-level web framework.

It sits above Axum and gives you:

- a consistent project structure
- built-in features like auth and CRUD
- tooling to generate and scale your app quickly

Think less boilerplate, more building.

## Generate a resource

```bash
rustwing g resource product \
  --fields 'title:string:required:length(1,255)' \
  --fields 'price:f64:required:range(0.0,9999.0)'
```

This generates:

- Domain model (`Product`)
- Request/response DTOs with validation
- Service functions that own validation, pagination limits, and business logic
- SQLx-native repository glue and explicit CRUD behavior
- Route handlers with offset and cursor pagination
- Router registration
- Database migration

Optional fields remain optional in create requests and inserts. Validation
rules apply to both create and update requests, and generated list queries use
explicit ordering.

## Generate a scoped resource

For SaaS and parent-child resources, keep single-tenant CRUD as the default and opt into scoped generation explicitly:

```bash
rustwing g resource ticket \
  --tenant org_id \
  --fields 'org_id:uuid:required' \
  --fields 'subject:string:required:length(1,255)' \
  --fields 'assigned_member_id:uuid:optional'
```

This generates nested routes like `/orgs/{org_id}/tickets`, plus scoped repository helpers such as `find_by_org_id`, `update_by_org_id_and_id`, and `delete_by_org_id_and_id`. Every item operation includes the tenant and entity identifiers directly in SQL.

Route and SQL scoping prevent accidental cross-scope queries, but applications
must still enforce tenant membership in their services. Rustwing does not treat
a caller-supplied `org_id` as proof of authorization.

Scopes are not limited to tenants:

```bash
rustwing g resource comment \
  --scope ticket_id \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

This generates routes like `/tickets/{ticket_id}/comments`. You can combine scopes, for example `--tenant org_id --scope ticket_id`, to generate routes like `/orgs/{org_id}/tickets/{ticket_id}/comments`.

## OpenAPI docs and TypeScript client

Rustwing generated apps expose API docs by default:

```txt
/openapi.json
/docs
/redoc
```

Run the API and open `http://localhost:3000/docs` to inspect and test generated endpoints.

You can also export the OpenAPI document and generate a frontend client:

```bash
rustwing g openapi
rustwing g client typescript
```

This writes `openapi/openapi.json` and `frontend/generated/{types.ts,client.ts,index.ts}`.

## Project structure

```
my_app/
├── api/                        # Web server (Axum)
│   ├── src/
│   │   ├── domain/             # Your data models
│   │   ├── http/               # Routes, handlers, DTOs
│   │   │   ├── dtos/           # Request/response types
│   │   │   └── handlers/       # Route handlers
│   │   ├── repository/         # SQLx-native database access
│   │   └── services/           # Business logic and orchestration
│   └── migrations/             # SQL migrations (auto-run, fail closed)
├── worker/                     # Durable PostgreSQL job worker
└── frontend/                   # (coming soon)
```

## Configuration

| Env var               | Required      | Default           | Description                                                       |
| --------------------- | ------------- | ----------------- | ----------------------------------------------------------------- |
| `APP_ENV`             | No            | `development`     | `development`, `test`, or strict `production`                     |
| `DATABASE_URL`        | Yes           | —                 | Postgres connection string                                        |
| `JWT_SECRET`          | Yes           | —                 | Strong, unique secret key for JWT tokens                          |
| `LLM_PROVIDER`        | No            | `stub`            | AI provider (`stub`, `deepseek`, `openai`, `gemini`, `anthropic`) |
| `LLM_MODEL`           | No            | provider default  | Model name for the selected provider                              |
| `DEEPSEEK_API_KEY`    | For DeepSeek  | —                 | API key for DeepSeek                                              |
| `OPENAI_API_KEY`      | For OpenAI    | —                 | API key for OpenAI                                                |
| `GEMINI_API_KEY`      | For Gemini    | —                 | API key for Google Gemini                                         |
| `ANTHROPIC_API_KEY`   | For Anthropic | —                 | API key for Anthropic                                             |
| `LLM_MAX_TOKENS`      | No            | —                 | Default max output tokens; override per-request in code          |
| `RUST_LOG`            | No            | `info,api=debug`  | Log level                                                         |
| `WORKER_TICK_SECONDS` | No            | `10`              | Worker polling interval                                           |

Production rejects placeholder/short JWT secrets, the development LLM stub,
unknown providers, and missing provider credentials. Secret values are never
included in startup logs.

## Reliability primitives

`JobQueue`, `Outbox`, and `IdempotencyStore` are small SQLx-native PostgreSQL
primitives. Jobs and outbox events use leases and `FOR UPDATE SKIP LOCKED`,
recover after worker crashes, and deliver at least once. Consumers must be
idempotent. Outbox records and business changes can share the same explicit
SQLx transaction by passing `&mut *tx`.

Cursor endpoints return opaque cursors and deterministic UUIDv7 ID ordering.
Entities retain explicit `created_at`; business time must use domain fields such
as `issued_at`, `paid_at`, or `occurred_at`.

> Never use `f32` or `f64` for monetary values. Financial applications should
> define explicit decimal-based money, currency, and exchange-rate types.

## Roadmap

See [ROADMAP.md](ROADMAP.md) for current and future work.

## Documentation

- [Why Rustwing](docs/why-rustwing.md)
- [Manifesto](MANIFESTO.md)
- [Getting started](docs/getting-started.md)
- [CLI reference](docs/cli-reference.md)
- [OpenAPI and TypeScript client](docs/openapi.md)
- [Architecture guide](docs/architecture.md)
- [Configuration reference](docs/configuration.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, workflow, and publishing instructions.

## Built on

Rustwing is built on [Axum](https://github.com/tokio-rs/axum), [SQLx](https://github.com/launchbadge/sqlx), [Rig](https://github.com/0xPlaygrounds/rig), and the [Tokio](https://tokio.rs) ecosystem. None of this would exist without those projects.

## License

MIT
