# Roadmap

## v0.2 — Polish & hardening

- [x] **Service-first generation** — generated resources now include `services/<name>_service.rs`; handlers call services instead of jumping straight to repositories.
- [x] **Executable worker scaffold** — generated workers now wire `PgPool`, `LlmRef`, `WorkerState`, and a configurable tick loop.
- [x] **Stub LLM provider polish** — `LLM_PROVIDER=stub` is a first-class provider and no longer logs an "unknown provider" warning.
- [x] **Scoped resource generation** — `rustwing g resource ... --scope ticket_id` and `--tenant org_id` generate nested routes and scoped SQLx repository helpers.
- [x] **Nullable PATCH helper** — `Nullable<T>` documents and supports missing/null/value update semantics.
- [ ] **Cursor pagination fix** — currently relies on UUID ordering (`WHERE id > $1`). Should use a dedicated `sort_order` column or `created_at` for predictable pagination.
- [ ] **`password_hash` isolation** — `User` domain model includes `password_hash` via `FromRow`. One `UserResponse::from()` bug away from leaking hashes into API responses. Either exclude it from the response conversion or use a separate DB-only struct.
- [ ] **Soft deletes** — add a `deleted_at` column pattern to `generic_crud`.
- [ ] **Rate limiting** — add `tower` middleware for per-endpoint rate limits.
- [ ] **Request ID tracing** — add a unique request ID to every log line.
- [ ] **Feature flags on `rustwing`** — `rig-core` (LLM) and `argon2` (auth) as optional features.

## v0.3 — Background jobs

- [ ] **Worker crate implementation** — Postgres-based job queue using `LISTEN/NOTIFY` + advisory locks. No Redis dependency.
- [ ] **`rustwing g job process_payment`** — scaffold a new background job type.
- [ ] **Job retry with backoff** — configurable retry strategy in the worker.

## v0.4 — Frontend SDK

- [ ] **`rustwing g resource` emits TypeScript types** — alongside the Rust code, generate a `typescript/types.ts` with interfaces for all DTOs.
- [ ] **OpenAPI spec generation** — derive or annotate routes to produce an OpenAPI document.
- [ ] **Generated API client** — TypeScript fetch client that matches the Rust API.

## v0.5 — SaaS features

- [ ] **Billing trait** — Stripe integration with idempotent webhook verification.
- [ ] **Email trait** — transactional email abstraction (SMTP / Sendgrid / Resend).
- [ ] **File storage trait** — S3-compatible storage (local fs in dev, S3 in prod).
- [ ] **Tenant isolation** — multi-tenant support baked into `generic_crud` filters.

## Future ideas

- Hot reload for development (`cargo watch` integration)
- WebSocket channel scaffold (`rustwing g channel chat`)
- Admin dashboard generator
- One-click deploy to Railway / Fly.io / Shuttle
