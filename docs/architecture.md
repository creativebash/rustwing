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
| `generic_crud::find_all::<T>` | `SELECT * FROM table LIMIT $1 OFFSET $2` | Offset |
| `generic_crud::find_after::<T>` | `SELECT * FROM table WHERE id > $1 ORDER BY id LIMIT $2` | Cursor |
| `generic_crud::find_by_id::<T>` | `SELECT * FROM table WHERE id = $1` | — |
| `generic_crud::insert::<T, I>` | `INSERT INTO table (...) VALUES (...) RETURNING *` | — |
| `generic_crud::update::<T, U>` | `UPDATE table SET ... WHERE id = $1 RETURNING *` | — |
| `generic_crud::delete::<T>` | `DELETE FROM table WHERE id = $1` | — |

## Auth

Authentication uses Argon2 for password hashing and JWT for session tokens. The `AuthUser` extractor enforces authentication on any handler:

```rust
pub async fn get_profile(
    auth: AuthUser,          // ← rejects unauthenticated requests
    State(state): State<AppState>,
) -> Result<Json<UserResponse>, AppError> {
    let user = generic_crud::find_by_id::<User>(&state.db, auth.id).await?;
    Ok(Json(UserResponse::from(user)))
}
```

Public routes (like `register` and `login`) omit the `AuthUser` parameter.

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
  --tenant organization_id \
  --fields 'organization_id:uuid:required' \
  --fields 'subject:string:required'
```

Scope fields must be required `uuid` or `ref` fields. Scopes can be repeated:

```bash
rustwing g resource note \
  --tenant organization_id \
  --scope ticket_id \
  --fields 'organization_id:uuid:required' \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

This generates routes like:

```
/organizations/{organization_id}/tickets/{ticket_id}/notes
/organizations/{organization_id}/tickets/{ticket_id}/notes/{id}
```

Generated repository helpers include scope filters on list, get, update, and delete operations, for example `find_by_organization_id_and_ticket_id` and `delete_by_organization_id_and_ticket_id_and_id`.

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
```

Built-in implementations:
- `DeepSeek` — production, requires `DEEPSEEK_API_KEY`
- `Stub` — local development, returns canned responses

Configure via environment variables:
- `LLM_PROVIDER` — `"stub"` or `"deepseek"`
- `LLM_MODEL` — model name (e.g. `"deepseek-chat"`)

## Worker

Generated projects include a `worker` binary with dotenv and tracing setup, `PgPool`, `LlmRef`, `WorkerState`, and a configurable tick loop via `WORKER_TICK_SECONDS`.

The default `process_pending_jobs` function is intentionally empty, but executable. Add polling, queue processing, or background AI workflows there.
