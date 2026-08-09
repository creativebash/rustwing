# Roadmap

Rustwing's next milestones prioritize trustworthy generated applications over
adding more integrations. A feature is complete only when the template,
generator, documentation, and representative generated-project tests agree.

The current dependency baseline is SQLx 0.9, Rig 0.38, and jsonwebtoken 11;
framework and generated-template manifests are upgraded together so generated
applications do not drift onto incompatible database or integration APIs.

## v0.2 — Safety and correctness

### Security and tenancy

- [x] **Self-only account routes** — the starter app exposes `/users/me`
  instead of allowing any authenticated user to read, update, or delete an
  arbitrary user ID.
- [x] **Password-record isolation** — password hashes live in a database-only
  `UserRecord`; public domain and response models cannot serialize them.
- [x] **Service-first auth** — registration and login validation,
  password work, SQL, and token creation live in the auth service.
- [x] **Async-safe password work** — Argon2 hashing and verification run on
  Tokio's blocking pool instead of blocking request executors.
- [x] **Required JWT configuration** — generated APIs fail at startup when
  `JWT_SECRET` is absent instead of using a shared fallback.
- [x] **Tenant membership authorization** — generated `--tenant` resources
  verify active organization membership before repository access; route/query
  scope alone is not an authorization boundary.
- [ ] **Auth lifecycle** — issuer/audience claims, key rotation, refresh
  sessions, revocation, password-hash upgrades, and login/register rate limits.

### Database and API correctness

- [x] **Migration immutability** — generated applications fail closed on
  missing or divergent applied migrations; startup never edits SQLx history.
- [x] **Optional create fields** — `optional` fields are optional in create
  DTOs and inserts as well as in domain/database models.
- [x] **Validated updates** — generated PATCH/PUT DTOs preserve field
  validators and services validate them before database writes.
- [x] **Datetime and numeric mappings** — user-defined datetime fields are
  generated normally, reserved framework fields are rejected, and `f64` maps
  to PostgreSQL `DOUBLE PRECISION`.
- [x] **Deterministic offset pagination** — generic and scoped list queries
  use explicit ID ordering.
- [x] **Scope indexes** — scoped resource migrations create indexes matching
  their generated filters.
- [x] **Stable cursor pagination** — UUIDv7 generation is consistent and
  same-millisecond monotonic within a process; endpoints expose opaque ID
  cursors with deterministic SQL ordering and explicit business timestamps.
- [ ] **Nullable PATCH generation** — generate `Nullable<T>` for nullable
  fields that clients must be able to clear.
- [ ] **Soft deletes** — add an explicit `deleted_at` generator pattern and
  CRUD filtering rules.

### Runtime hardening

- [x] **Typed startup configuration** — validate environment-specific
  settings, pool sizes, ports, secrets, and provider configuration together.
- [x] **HTTP request observability and shutdown** — request IDs, structured
  tracing, response propagation, sensitive-data logging rules, and graceful shutdown.
- [x] **Rate limiting** — process-local global and sensitive-endpoint policies.
- [x] **Health model** — separate liveness and PostgreSQL-aware readiness.
- [ ] **Feature flags** — make auth and LLM dependencies optional so minimal
  applications do not compile unused provider stacks.
- [x] **LLM failure safety** — invalid providers/configuration fail
  startup; prompts must not be logged by default.

### Verification

- [ ] **PostgreSQL integration suite** — execute generated migrations and
  CRUD operations for every supported field type.
- [ ] **HTTP security suite** — cover authentication, self-only accounts,
  tenant isolation, validation, malformed input, and conflict responses.
- [ ] **Generator golden tests** — snapshot normal, optional, scoped,
  multi-scoped, model-only, and failure cases.
- [ ] **Contract tests** — validate exported OpenAPI and compile generated
  TypeScript with `tsc --noEmit`.
- [ ] **Supply-chain checks** — add dependency policy/advisory checks and
  semantic-version compatibility checks to releases.

## v0.3 — Generator lifecycle and upgrades

- [ ] **Structured resource specification** — parse CLI input into one typed
  intermediate representation used by Rust, SQL, OpenAPI, and client output.
- [ ] **Atomic generation** — stage and validate the complete change set
  before modifying an application.
- [ ] **`rustwing g ... --dry-run` and `--json`** — make generated plans
  inspectable by people, CI, and coding agents.
- [x] **Scaffold version metadata** — record which template version created
  an application in `.rustwing-version`.
- [x] **`rustwing doctor`** — diagnose scaffold version, dependency wiring,
  migration structure, and required configuration drift.
- [x] **Upgrade assistance** — provide focused diagnostics and a verified,
  non-destructive framework lockfile upgrade without taking ownership away
  from applications.
- [ ] **Typed Clap subcommands** — replace stringly generator dispatch with
  testable command types returning normal `Result` values.

## v0.4 — Reliable background jobs

- [x] **PostgreSQL job queue** — durable claims using PostgreSQL locking
  primitives, without requiring Redis.
- [ ] **`rustwing g job process_payment`** — scaffold typed job payloads and
  handlers.
- [x] **Retries and dead letters** — bounded exponential backoff, attempt
  history, lease expiry, crash recovery, and dead-letter handling.
- [x] **Transactional outbox** — enqueue work in the same transaction as
  application state changes.
- [ ] **Job observability** — tracing, metrics, queue depth, latency, and
  failure inspection.

## v0.5 — Contract and client ecosystem

- [x] **OpenAPI spec generation** — generated apps expose `/openapi.json`,
  `/docs`, and `/redoc`.
- [x] **Generated TypeScript client** — `rustwing g client typescript` emits
  a typed fetch client from the runtime contract.
- [ ] **Client compatibility gates** — detect breaking OpenAPI changes in CI.
- [ ] **Streaming and realtime patterns** — documented/generated SSE,
  WebSocket, and streaming-response slices where Axum remains visible.

## Later — Optional application batteries

These should follow the safety, upgrade, and job foundations:

- Billing and idempotent webhook verification
- Transactional email providers
- S3-compatible file storage
- Deployment presets for common Rust hosts
- Admin interfaces built on stable public contracts
- LLM streaming, tool calls, structured output, and provider telemetry

## Non-goals

Rustwing will not become an ORM, hide Axum, introduce a private query
language, or require a proprietary runtime. Generated code remains explicit,
editable, SQLx-native application code.
