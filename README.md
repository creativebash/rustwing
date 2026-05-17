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
- **Service-first CRUD scaffolding** — Generate REST endpoints, services, repositories, and migrations
- **Scoped resources** — Opt into SaaS-style or parent-child routes and SQLx helpers with `--tenant` or `--scope`
- **Migrations** — Automatic database migrations on run
- **Background workers** — Wired worker binary with DB pool, LLM client, and tick loop
- **LLM hooks** — Pluggable AI integrations (DeepSeek, local stubs)
- **Scaffolding CLI** — Generate resources, models, services, repositories, handlers, and routes instantly
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

## Generate a scoped resource

For SaaS and parent-child resources, keep single-tenant CRUD as the default and opt into scoped generation explicitly:

```bash
rustwing g resource ticket \
  --tenant org_id \
  --fields 'org_id:uuid:required' \
  --fields 'subject:string:required:length(1,255)' \
  --fields 'assigned_member_id:uuid:optional'
```

This generates nested routes like `/orgs/{org_id}/tickets`, plus scoped repository helpers such as `find_by_org_id`, `update_by_org_id_and_id`, and `delete_by_org_id_and_id`.

Scopes are not limited to tenants:

```bash
rustwing g resource comment \
  --scope ticket_id \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

This generates routes like `/tickets/{ticket_id}/comments`. You can combine scopes, for example `--tenant org_id --scope ticket_id`, to generate routes like `/orgs/{org_id}/tickets/{ticket_id}/comments`.

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
│   └── migrations/             # SQL migrations (auto-run)
├── worker/                     # DB/LLM-backed worker tick loop
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
| `WORKER_TICK_SECONDS` | No | `10` | Worker polling interval |

## Roadmap

See [ROADMAP.md](ROADMAP.md) for what's coming — job queues, frontend SDK generation, billing integration, and more.

## Documentation

- [Manifesto](MANIFESTO.md)
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
