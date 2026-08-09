# Architecture

## Prelude

```rust
use rustwing::prelude::*;
```

Brings in the most common framework items:
- `CoreError` — error types
- `AuthEngine` — password hashing and JWT
- `LlmRef`, `LlmRequest`, `LlmResponse` — LLM types
- `Nullable` — PATCH helper for missing/null/value update fields
- `generic_crud` — CRUD functions module
- `ModelName`, `Insertable`, `Updateable`, `UpdateResult` — repository traits

## Layers

```
┌─────────────────────────────────────────────┐
│               HTTP Layer                     │
│  routes → handlers (AuthUser) → DTOs        │
│  OpenAPI JSON + Swagger/ReDoc docs          │
├─────────────────────────────────────────────┤
│             Service Layer                    │
│  validation, tenant scope, AI calls          │
├─────────────────────────────────────────────┤
│            Repository Layer                  │
│  trait impls (ModelName, Insertable, etc.)   │
├─────────────────────────────────────────────┤
│          rustwing (framework)                │
│  generic_crud, auth, LLM, error types        │
├─────────────────────────────────────────────┤
│               Database                       │
│  PostgreSQL, migrations via sqlx             │
└─────────────────────────────────────────────┘
```

## Generic CRUD

The framework provides generic CRUD operations that work with any model. You implement three traits:

### `ModelName`

Maps a domain struct to its database table:

```rust
impl ModelName for Product {
    fn table_name() -> &'static str { "products" }
}
```

### `Insertable` (on insert payload)

Defines columns and values for INSERT queries:

```rust
impl Insertable for InsertProduct {
    fn columns() -> Vec<&'static str> {
        vec!["title", "price"]
    }
    fn bind_values<'a>(&'a self, query: &mut QueryBuilder<'a, Postgres>) {
        let mut separated = query.separated(", ");
        separated.push_bind(&self.title);
        separated.push_bind(self.price);
    }
}
```

### `Updateable` (on update payload)

Defines partial update logic — only binds non-None fields:

```rust
impl Updateable for ProductUpdate {
    fn bind_updates<'a>(&'a self, query: &mut QueryBuilder<'a, Postgres>) -> UpdateResult {
        let mut separated = query.separated(", ");
        let mut has_updates = false;
        if let Some(ref v) = self.title {
            separated.push("title = ").push_bind_unseparated(v);
            has_updates = true;
        }
        if let Some(v) = self.price {
            separated.push("price = ").push_bind_unseparated(v);
            has_updates = true;
        }
        if has_updates { UpdateResult::HasUpdates } else { UpdateResult::NoChanges }
    }
}
```

### Available operations

| Function | SQL | Pagination |
|---|---|---|
| `generic_crud::find_all::<T>` | `SELECT * FROM table ORDER BY id LIMIT $1 OFFSET $2` | Offset |
| `generic_crud::find_after::<T>` | `SELECT * FROM table WHERE id > $1 ORDER BY id LIMIT $2` | Cursor |
| `generic_crud::find_by_id::<T>` | `SELECT * FROM table WHERE id = $1` | — |
| `generic_crud::insert::<T, I>` | `INSERT INTO table (...) VALUES (...) RETURNING *` | — |
| `generic_crud::update::<T, U>` | `UPDATE table SET ... WHERE id = $1 RETURNING *` | — |
| `generic_crud::delete::<T>` | `DELETE FROM table WHERE id = $1` | — |

These functions accept SQLx PostgreSQL executors. Pass `&pool` for ordinary
operations or `&mut *tx` to compose multiple repositories explicitly inside a
service-owned transaction. Rustwing does not add a Unit-of-Work abstraction.

## Auth

Authentication uses Argon2 for password hashing and JWT for session tokens.
Argon2 work runs on Tokio's blocking pool. Password hashes live in a
database-only `UserRecord`, while `User` and `UserResponse` contain public
account data. The `AuthUser` extractor enforces authentication on handlers:

```rust
pub async fn get_profile(
    auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<UserResponse>, AppError> {
    let user = generic_crud::find_by_id::<User>(&state.db, auth.id).await?;
    Ok(Json(UserResponse::from(user)))
}
```

Public routes (like `register` and `login`) omit the `AuthUser` parameter. The
starter exposes self-only `/users/me` operations. Authentication alone is
never used as permission to operate on an arbitrary user ID.

## Services

Generated resources use a service-first flow:

```
Handler → service → repository/generic_crud → database
```

Handlers should stay thin: extract auth, route params, query params, and JSON, then call a service. Services own validation, pagination normalization, tenant scoping, side effects, LLM calls, and orchestration.

For a normal single-tenant resource, generated services call `generic_crud` directly. For scoped resources, generated services call scoped SQLx repository helpers.

## Scoped Resources

Single-tenant CRUD is the default. Resources can opt into route and SQL scope with `--scope`:

```bash
rustwing g resource comment \
  --scope ticket_id \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

This generates nested routes like `/tickets/{ticket_id}/comments`. Scope fields come from the route path, not the request body.

`--tenant` is a convenience alias for the common SaaS tenant scope:

```bash
rustwing g resource ticket \
  --tenant org_id \
  --fields 'org_id:uuid:required' \
  --fields 'subject:string:required'
```

Scope fields must be required `uuid` or `ref` fields. Scopes can be repeated:

```bash
rustwing g resource note \
  --tenant org_id \
  --scope ticket_id \
  --fields 'org_id:uuid:required' \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

This generates routes like:

```
/orgs/{org_id}/tickets/{ticket_id}/notes
/orgs/{org_id}/tickets/{ticket_id}/notes/{id}
```

Generated repository helpers include scope filters on list, get, update, and delete operations, for example `find_by_org_id_and_ticket_id` and `delete_by_org_id_and_ticket_id_and_id`.

Scoped migrations also create a composite index matching the generated scope
filter. Scope remains a data-access boundary, not proof of membership:
services must authorize the current actor for every tenant or parent scope.

Scope fields are excluded from writable JSON DTOs. For combined tenant and
parent scopes, every generated item query contains the tenant ID, parent ID,
and entity ID. Generic unscoped CRUD remains available only for global data.

## UUIDv7 and cursor pagination

Rustwing uses `uuid` 1.24 and `Uuid::now_v7()`. Its shared `ContextV7`
guarantees creation ordering within a process, including UUIDs generated in
the same millisecond. Generated inserts supply IDs from Rust; migrations do
not fall back to PostgreSQL UUIDv4 defaults.

Cursor endpoints order by UUIDv7 `id`, request a versioned opaque base64url
cursor, and return `{ items, next_cursor }`. This is deterministic for a fixed
result set but is not snapshot isolation under concurrent inserts. UUID time
is never the canonical business timestamp: keep `created_at` and explicit
fields such as `issued_at`, `paid_at`, or `occurred_at`.

## Request context and health

Every generated API request receives an `X-Request-ID`. A safe incoming value
is preserved; otherwise a UUIDv7 value is generated. It is returned in the
response, stored in request extensions, and attached to the structured tracing
span. Jobs and outbox events accept the same correlation ID.

`/health/live` only reports that the process is running. `/health/ready` uses a
short bounded PostgreSQL query and returns 503 while the database is unavailable.

## Durable jobs

`JobQueue` stores JSON jobs in PostgreSQL. Claiming uses `FOR UPDATE SKIP
LOCKED`, leases, worker ownership, and bounded attempt counts. Active jobs
heartbeat; expired leases can be claimed by another process. Retryable errors
use bounded exponential backoff, while malformed/unknown jobs go directly to
the visible `DEAD` state. Shutdown stops new claims and lets the current batch
finish. Delivery is at-least-once.

Application-specific job names and typed payloads remain in the worker. Do not
log full payloads because they may contain confidential documents or provider data.

## Transactional outbox

Call `Outbox::record(&mut *tx, event)` inside the same SQLx transaction as the
business mutation. Dispatch uses stable event IDs, leases, retries, and a
recorded `dispatched_at`. A crash after an external effect but before marking
success can redeliver the event, so consumers must use the event ID idempotently.

## Idempotency

`IdempotencyStore::process_once` namespaces keys by provider/workflow and
optional organization, verifies a request fingerprint, serializes concurrent
duplicates with a row lock, and stores successful JSON results for replay. The
business closure runs in a savepoint: failures roll back its writes but retain
an inspectable retry state. A process crash rolls back the outer transaction
and cannot leave a permanent processing lock.

For outgoing provider calls, reuse the same provider idempotency key after an
ambiguous timeout. Rustwing cannot determine whether the remote provider
completed an operation.

## Safe webhook flow

Use ordinary Axum extraction in this order:

```text
raw axum body bytes
→ verify the provider signature over the exact bytes
→ extract external event ID and request fingerprint
→ IdempotencyStore in an application service
→ domain mutation plus Outbox::record in one SQLx transaction
→ deliberately minimal HTTP response
```

Do not deserialize or trust payload fields before signature verification.
Request context remains available through extensions throughout this flow.

## Database constraints and money

Generated migrations are normal explicit PostgreSQL SQL. Add foreign keys,
`UNIQUE`, compound `UNIQUE`, `CHECK`, and matching indexes directly before the
migration is first applied. Migration ordering is numeric and startup fails on
missing, divergent, or failed migrations.

Never represent monetary values with `f32` or `f64`. Rustwing retains `f64` for
ordinary measurements; financial applications must define decimal-backed
money, currency, and exchange-rate domain types.

## OpenAPI

Generated apps expose:

```txt
/openapi.json
/docs
/redoc
```

Rustwing uses generated DTO derives and `utoipa::path` annotations in handlers. The resource generator injects new generated routes and schemas into `api/src/openapi.rs`, so normal generated resources do not require hand-written OpenAPI specs.

`rustwing g openapi` exports the runtime OpenAPI document to `openapi/openapi.json`, and `rustwing g client typescript` generates a typed fetch client in `frontend/generated/`.

Scoped resources document path params automatically. For example, `--tenant org_id --scope ticket_id` produces paths like `/orgs/{org_id}/tickets/{ticket_id}/notes/{note_id}` with `org_id`, `ticket_id`, and `note_id` documented as UUID path parameters.

## Error handling

Errors flow through a consistent chain:

```
sqlx::Error / validation errors
    → CoreError (rustwing)
        → AppError (api crate)
            → JSON response with appropriate HTTP status
```

PostgreSQL error codes are mapped:
- `23505` (unique violation) → `409 Conflict`
- `23503` (foreign key violation) → `409 Conflict`
- All others → `500 Internal Server Error`

## Nullable PATCH Fields

Plain `Option<T>` update fields are fine when `None` means "do not change". For nullable columns where clients must be able to clear a value, use `Nullable<T>`:

```rust
#[derive(Deserialize)]
pub struct UpdateTicket {
    #[serde(default)]
    pub assigned_member_id: Nullable<Uuid>,
}
```

Interpretation:
- `Nullable::Missing` — field absent, do not update
- `Nullable::Null` — JSON `null`, write SQL `NULL`
- `Nullable::Value(value)` — write the provided value

In `Updateable`, bind `Nullable::Null` with a typed `None`:

```rust
match &self.assigned_member_id {
    Nullable::Missing => {}
    Nullable::Null => {
        separated.push("assigned_member_id = ").push_bind_unseparated(Option::<Uuid>::None);
    }
    Nullable::Value(id) => {
        separated.push("assigned_member_id = ").push_bind_unseparated(id);
    }
}
```

This pattern is intentionally explicit because not every optional database column needs clear-via-PATCH behavior.

## LLM integration

The LLM system uses a trait-based abstraction:

```rust
pub trait Llm: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, CoreError>;
}

pub struct LlmRequest {
    pub prompt: String,
    pub max_tokens: Option<u32>,  // per-request override; None = use agent default
}

pub struct LlmResponse {
    pub completion: String,
    pub usage: Option<LlmUsage>,
    pub provider: String,
    pub model: String,
    pub request_id: Option<String>,
    pub latency_ms: u64,
}

// Token counts are normalized across providers. Usage may be unavailable.
pub struct LlmUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    // Cache, tool-use, and reasoning token fields are also available.
}
```

Built-in implementations:
- `DeepSeek` — requires `DEEPSEEK_API_KEY`
- `OpenAI` — requires `OPENAI_API_KEY`
- `Gemini` — requires `GEMINI_API_KEY`
- `Anthropic` — requires `ANTHROPIC_API_KEY`
- `Stub` — local development, returns canned responses without usage metrics

Configure via environment variables:
- `LLM_PROVIDER` — `"stub"`, `"deepseek"`, `"openai"`, `"gemini"`, or `"anthropic"`
- `LLM_MODEL` — optional model name override; if unset, Rustwing uses the selected provider's default model
- `LLM_MAX_TOKENS` — optional default max output tokens; set per-request via `LlmRequest.max_tokens`

Provider responses expose normalized token usage when the provider reports it,
along with provider/model metadata, request ID when available, and request
latency. Prompt and completion content is not logged by the framework.

Completions remain provider-neutral text. Applications should deserialize and
validate structured output in a service before invoking typed application
tools or domain operations. LLM code must never receive an arbitrary database
mutation path.

## Worker

Generated projects include a durable PostgreSQL worker with structured tracing,
leased claims, heartbeats, bounded retries, dead jobs, correlation fields, and
graceful shutdown. Add application-owned job matching and validated payload
deserialization in `handle_job`.

## Hardening upgrade notes

Applications created before this hardening release require deliberate changes:

- cursor query parameters are opaque strings and cursor responses are page
  envelopes rather than bare arrays;
- inserts must supply UUIDv7 IDs because generated migrations no longer use a
  UUIDv4 database default;
- `build_client` and `build_client_with_config` return `Result` and never
  silently replace a configured provider with the stub;
- copy/adapt the jobs, outbox, and idempotency migration only as a new,
  immutable migration; never rewrite one already applied in production.
