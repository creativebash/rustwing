# Architecture

## Layers

```
┌─────────────────────────────────────────────┐
│               HTTP Layer                     │
│  routes → handlers (AuthUser) → DTOs        │
├─────────────────────────────────────────────┤
│             Service Layer                    │
│  business logic, AI calls, orchestration     │
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
- `LLM_PROVIDER` — `"deepseek"` or `"stub"`
- `LLM_MODEL` — model name (e.g. `"deepseek-chat"`)
