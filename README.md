<div align="center">

# Rustwing

**Full-stack Rust SaaS framework built on Axum — auth, CRUD, LLM, background jobs**

[![Crates.io](https://img.shields.io/crates/v/rustwing.svg)](https://crates.io/crates/rustwing)
[![Crates.io](https://img.shields.io/crates/v/rustwing-cli.svg)](https://crates.io/crates/rustwing-cli)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

```bash
cargo install rustwing-cli
rustwing new my_app
cd my_app
cargo run --bin api
```

</div>

## Why Rustwing?

SaaS applications share ~80% boilerplate — auth, CRUD, database migrations, background jobs, LLM integration. Rustwing bakes those in so you only write the 20% that makes your app unique.

Rust + Axum + PostgreSQL. Opinionated, extensible, compiled.

## What you get

- **Auth** — Argon2 password hashing + JWT tokens, ready out of the box
- **Generic CRUD** — implement 3 traits, get full REST endpoints without writing SQL
- **LLM integration** — pluggable AI backend (DeepSeek, stub for local dev)
- **Scaffolding** — `rustwing g resource post title:string body:text` generates model, DTOs, repo glue, handlers, router, migration
- **Background worker** — skeleton crate ready for async job processing
- **Migrations** — run automatically on `cargo run`, no manual steps
- **Error handling** — PG error code mapping (unique violations → 409, not 500)

## Quick start

```bash
# Install
cargo install rustwing-cli

# Create a project
rustwing new my_saas
cd my_saas

# Configure your database
cp .env.example .env
# Edit DATABASE_URL in .env

# Start the API server — creates tables automatically
cargo run --bin api
```

## Generate a resource

```bash
rustwing g resource product \
  --fields 'title:string:required:length(1,255)' \
  --fields 'price:f64:required:range(0.0,9999.0)'
```

This generates:
- Domain model (`Product`)
- Request/response DTOs with validation
- Repository glue (zero-SQL CRUD via traits)
- Route handlers with offset and cursor pagination
- Router registration
- Database migration

## Project structure

```
my_app/
├── api/                        # Web server (Axum)
│   ├── src/
│   │   ├── domain/             # Your data models
│   │   ├── http/               # Routes, handlers, DTOs
│   │   │   ├── dtos/           # Request/response types
│   │   │   └── handlers/       # Route handlers
│   │   ├── repository/         # DB trait implementations
│   │   └── services/           # Business logic
│   └── migrations/             # SQL migrations (auto-run)
├── worker/                     # Background job worker
└── frontend/                   # (coming soon)
```

## Configuration

| Env var | Required | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | Yes | — | Postgres connection string |
| `JWT_SECRET` | No | dev-only fallback | Secret key for JWT tokens |
| `LLM_PROVIDER` | No | `stub` | AI provider (`stub`, `deepseek`) |
| `LLM_MODEL` | No | `deepseek-chat` | Model name for the provider |
| `RUST_LOG` | No | `info,api=debug` | Log level |

## Roadmap

See [ROADMAP.md](ROADMAP.md) for what's coming — background jobs, frontend SDK generation, billing integration, and more.

## Documentation

- [Getting started](docs/getting-started.md)
- [CLI reference](docs/cli-reference.md)
- [Architecture guide](docs/architecture.md)
- [Configuration reference](docs/configuration.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, workflow, and publishing instructions.

## Built on

Rustwing is built on [Axum](https://github.com/tokio-rs/axum), [SQLx](https://github.com/launchbadge/sqlx), [Rig](https://github.com/0xPlaygrounds/rig), and the [Tokio](https://tokio.rs) ecosystem. None of this would exist without those projects.

## License

MIT
