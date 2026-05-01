<div align="center">

# Rustwing

**The application framework for building Rust apps with Axum.**

Rustwing is a batteries-included application framework that helps you build production-ready Rust web apps faster. It provides structure, conventions, and ready-made features on top of Axum, so you can focus on your product instead of boilerplate.

[![Crates.io](https://img.shields.io/crates/v/rustwing.svg)](https://crates.io/crates/rustwing)
[![Crates.io](https://img.shields.io/crates/v/rustwing-cli.svg)](https://crates.io/crates/rustwing-cli)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

</div>

## What you get

- **Auth** — Argon2 password hashing + JWT tokens out of the box
- **CRUD scaffolding** — Generate full REST endpoints without writing SQL
- **Migrations** — Automatic database migrations on run
- **Background workers** — Built-in structure for async job processing
- **LLM hooks** — Pluggable AI integrations (DeepSeek, local stubs)
- **Scaffolding CLI** — Generate resources, models, handlers, and routes instantly
- **Error handling** — Clean mapping of database and application errors

## Quick start

```bash
cargo install rustwing-cli
rustwing new my_app
cd my_app
rustwing run
```

## Philosophy

Rustwing is not a replacement for Axum — it builds on top of it.

- **Axum handles HTTP**
- **Rustwing handles your application**

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
