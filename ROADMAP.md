# Roadmap

## Pre-launch checklist

These are the items I want polished before the public announcement.

### Before publishing

- [ ] **Register domains** — `rustwing.rs` (primary), `rustwing.com`, `rustwing.dev` (redirects)
- [x] **`rustwing::prelude` module** — one `use rustwing::prelude::*;` brings in `generic_crud`, `AuthEngine`, `CoreError`, `ModelName`, `Insertable`, `Updateable`.
- [x] **Auth middleware** — `AuthUser` extractor with JSON error responses. All generated handlers require auth by default.
- [ ] **Service layer consistency** — `Post` routes skip the service layer entirely. Generated resources should always go through a `XxxService`. Makes framework-level hooks (audit, events) possible.
- [ ] **Feature flags on `rustwing`** — `rig-core` (LLM) and `argon2` (auth) are heavy deps. Users building a pure CRUD API shouldn't compile DeepSeek SDK. Make them optional:

```toml
[dependencies]
rustwing = { version = "0.1", default-features = false, features = ["auth"] }
```

- [ ] **Template prelude in generated projects** — `api/src/main.rs` currently has explicit imports. A generated `prelude.rs` or re-export module would clean it up.
- [ ] **Error messages** — Review all `eprintln!` and `expect()` messages in the CLI. First-time users should get clear, actionable guidance.

### For launch day

- [ ] **Publish `rustwing` to crates.io**
- [ ] **Publish `rustwing-cli` to crates.io**
- [ ] **Create GitHub repo** — `github.com/creativebash/rustwing`
- [ ] **Push code**
- [ ] **Verify end-to-end**:
  ```bash
  cargo install rustwing-cli
  rustwing new test_drive
  cd test_drive
  # set up postgres
  cargo run --bin api        # should start, run migrations, listen on :3000
  curl -X POST http://localhost:3000/auth/register -H 'Content-Type: application/json' -d '{"username":"demo","email":"demo@test.com","password":"password123"}'
  ```
- [ ] **Post to socials** — Reddit (r/rust), Hacker News, Lobsters, Twitter/X
- [ ] **Write launch blog post** — explain the philosophy, show the scaffold in action, link to repo

## Post-launch roadmap

### v0.2 — Polish & hardening

- [ ] **Cursor pagination fix** — currently relies on UUID ordering (`WHERE id > $1`). Should use a dedicated `sort_order` column or `created_at` for predictable pagination.
- [ ] **`password_hash` isolation** — `User` domain model includes `password_hash` via `FromRow`. One `UserResponse::from()` bug away from leaking hashes into API responses. Either exclude it from the response conversion or use a separate DB-only struct.
- [ ] **Soft deletes** — add a `deleted_at` column pattern to `generic_crud`.
- [ ] **Rate limiting** — add `tower` middleware for per-endpoint rate limits.
- [ ] **Request ID tracing** — add a unique request ID to every log line.

### v0.3 — Background jobs

- [ ] **Worker crate implementation** — Postgres-based job queue using `LISTEN/NOTIFY` + advisory locks. No Redis dependency.
- [ ] **`rustwing g job process_payment`** — scaffold a new background job type.
- [ ] **Job retry with backoff** — configurable retry strategy in the worker.

### v0.4 — Frontend SDK

- [ ] **`rustwing g resource` emits TypeScript types** — alongside the Rust code, generate a `typescript/types.ts` with interfaces for all DTOs.
- [ ] **OpenAPI spec generation** — derive or annotate routes to produce an OpenAPI document.
- [ ] **Generated API client** — TypeScript fetch client that matches the Rust API.

### v0.5 — SaaS features

- [ ] **Billing trait** — Stripe integration with idempotent webhook verification.
- [ ] **Email trait** — transactional email abstraction (SMTP / Sendgrid / Resend).
- [ ] **File storage trait** — S3-compatible storage (local fs in dev, S3 in prod).
- [ ] **Tenant isolation** — multi-tenant support baked into `generic_crud` filters.

### Future ideas

- Hot reload for development (`cargo watch` integration)
- WebSocket channel scaffold (`rustwing g channel chat`)
- Admin dashboard generator
- One-click deploy to Railway / Fly.io / Shuttle
