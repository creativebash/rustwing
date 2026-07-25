# Rustwing project — for AI coding assistants

This is a Rustwing SaaS project. Below is the context you need to understand its structure and conventions before making changes or adding features.

## Agent operating rules

- Treat the `rustwing` CLI as the fastest and most reliable way to create app structure. Before hand-writing a CRUD resource, model, route bundle, DTO set, repository, service, or migration, check whether the request maps to `rustwing g resource` or `rustwing g model`.
- Prefer generating first, then editing the generated files for domain-specific behavior. This keeps routing, pagination, module registration, timestamps, migrations, validation shape, and repository traits aligned with Rustwing conventions.
- Use `rustwing g --help` when command syntax is unclear. The generator shape is `rustwing g <resource|model> <name> [--tenant ...] [--scope ...] --fields ...`.
- Use singular, snake_case names for generated resources and models, for example `ticket`, `comment`, `knowledge_base_article`.
- After generation, inspect the diff or changed files, add custom business rules in services first, and run `cargo check`.
- For token efficiency, do not expand or rewrite standard generated CRUD boilerplate in chat; run the CLI, then inspect only the changed files that need customization.
- Only fall back to manual scaffolding when the CLI is unavailable or cannot express the required shape. If that happens, follow the manual checklist below and keep the same file names, module registrations, and migration conventions the generator would have used.

## Project structure

```
├── api/                        # Web server (Axum)
│   ├── src/
│   │   ├── main.rs             # Entrypoint — connects DB, runs migrations, starts server
│   │   ├── openapi.rs          # OpenAPI document registration for generated routes
│   │   ├── state.rs            # AppState (db pool, LLM client, jwt_secret)
│   │   ├── error.rs            # AppError enum (wraps CoreError + validation)
│   │   ├── domain/             # Data models (one file per model)
│   │   │   ├── mod.rs
│   │   │   ├── user.rs
│   │   │   └── ...             # Generated resource models go here
│   │   ├── repository/         # ModelName trait impls + optional Insertable/Updateable
│   │   │   ├── mod.rs
│   │   │   ├── user_repo.rs
│   │   │   └── ...             # Generated resource repos go here
│   │   ├── http/
│   │   │   ├── mod.rs          # app_router() — all routes registered here
│   │   │   ├── extractors.rs   # AuthUser (JWT auth extractor)
│   │   │   ├── dtos/           # Request/response types per resource
│   │   │   │   ├── mod.rs
│   │   │   │   ├── user_dto.rs
│   │   │   │   └── ...         # Generated resource DTOs go here
│   │   │   └── handlers/       # Route handlers per resource
│   │   │       ├── mod.rs
│   │   │       ├── root.rs     # GET / health check
│   │   │       ├── auth_routes.rs  # POST /auth/register, POST /auth/login
│   │   │       ├── user_routes.rs  # Self-only account endpoints
│   │   │       └── ...         # Generated resource handlers go here
│   │   └── services/           # Business logic, validation, tenant scope, orchestration
│   │       ├── mod.rs
│   │       ├── auth_service.rs
│   │       ├── user_service.rs
│   │       └── ...             # Generated resource services go here
│   └── migrations/             # SQL migration files (auto-run on startup)
│       ├── 00000000000000_create_trigger_function.sql
│       └── ...
├── worker/                     # Background job worker with DB pool, LLM client, tick loop
└── frontend/                   # Your frontend (BYO)
```

## Important: Rustwing extends Axum, it does not replace it

Rustwing provides conventions, scaffolding, and built-in auth/CRUD on top of **Axum**. It does not abstract Axum away. For custom routes, middleware, extractors, or any non-trivial logic, you write raw Axum code using the same patterns as any Axum app. Familiarity with Axum, SQLx, and the broader Rust ecosystem is essential.

## Key conventions

### Resource generator - CLI-first

Whenever possible, generate new resources via the CLI rather than writing boilerplate. For agents, this saves tokens and avoids subtle wiring mistakes:

```
rustwing g resource product \
  --fields 'title:string:required:length(1,255)' \
  --fields 'price:f64:required:range(0.0,9999.0)'
```

This generates: domain model, DTOs (Create/Update/Response), service module, repository impl, route handlers (list/create/get/update/delete with pagination), router injection, OpenAPI metadata, and a database migration — all following the project's conventions.

Field syntax:

```
--fields 'name:type:required|optional[:validator]'
```

Supported field types: `string`, `text`, `int`/`i32`, `i64`, `float`/`f64`, `bool`, `uuid`, `datetime`, `json`/`jsonb`, `ref`.

Useful validator hints: `length(min,max)`, `range(min,max)`, `email`, `url`, `none`, or a raw `validator` crate expression. If no validator is supplied, strings, integers, and floats get sensible defaults.

Use `ref` for foreign key IDs when you want a generated `UUID REFERENCES <plural_table> (id)` column:

```
rustwing g resource invoice \
  --fields 'account_id:ref:required' \
  --fields 'amount:f64:required:range(0.0)'
```

For resources scoped to a parent record, use `--scope` explicitly:

```
rustwing g resource comment \
  --scope ticket_id \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

This generates nested routes like `/tickets/{ticket_id}/comments`. The scope field must be present in `--fields`, must be required, and must be `uuid` or `ref`. Create/update request bodies do not include scope fields; handlers take them from the route path and pass them into the service.

For SaaS tenant scope, use `--tenant`, which behaves like a first `--scope`:

```
rustwing g resource ticket \
  --tenant org_id \
  --fields 'org_id:uuid:required' \
  --fields 'subject:string:required:length(1,255)'
```

Scopes can be combined:

```
rustwing g resource note \
  --tenant org_id \
  --scope ticket_id \
  --fields 'org_id:uuid:required' \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

This generates routes like `/orgs/{org_id}/tickets/{ticket_id}/notes` and SQLx helpers such as `find_by_org_id_and_ticket_id`.

For data-only models without HTTP endpoints:

```
rustwing g model tag --fields 'name:string:required'
```

### OpenAPI and frontend client

Generated apps expose `/openapi.json`, `/docs`, and `/redoc` by default. For generated resources, the CLI updates `api/src/openapi.rs` and adds `utoipa::path` annotations so the docs stay in sync with routes, DTOs, auth, pagination, and scope params.

Use:

```bash
rustwing g openapi                 # writes openapi/openapi.json
rustwing g openapi --check         # CI drift check
rustwing g client typescript       # writes frontend/generated/
```

For custom routes, add `utoipa::path` annotations and register the handler/schema in `api/src/openapi.rs`.

### Auth pattern

- Protected routes use the `AuthUser` extractor, which validates the JWT.
- To add auth to a route, add `_auth: AuthUser` as the first parameter.
- The auth service generates JWTs with `AuthEngine::create_jwt(user_id, &jwt_secret)`; tokens have a 24-hour expiry.
- For login errors, always return a generic `401 Unauthorized` (not "wrong password") to prevent user enumeration.
- Keep password hashes in database-only records. Do not add them to serializable domain models or OpenAPI schemas.
- Starter account routes are self-only (`/users/me`). Any admin, directory, or cross-account operation requires an explicit authorization policy.
- A tenant path parameter is not proof of membership. Verify the authenticated actor's tenant access in the service before using scoped repository helpers.
- Never delete or rewrite rows in `_sqlx_migrations` to repair drift. Applied migrations are immutable; restore the missing file or perform an explicit, reviewed database repair.

### CRUD layer

```
DTO (Create/Update) → Handler → Service → Repository/generic_crud → DB
Domain model (FromRow) ← Handler ← Service ← Repository/generic_crud ← DB
```

- **Domain models**: `#[derive(Debug, Serialize, FromRow, Clone)]` — always include `id: Uuid`, `created_at: DateTime<Utc>`, `updated_at: DateTime<Utc>`.
- **Repositories**: Implement `ModelName` trait (just `table_name()`). For resources, also implement `Insertable` and `Updateable` traits.
- **DTOs**: Three types per resource — `Create{Name}` (with validation), `Update{Name}` (all optional and validated), `{Name}Response` (with `From<{Name}>` impl).
- **Services**: Own validation, pagination normalization, tenant scope, side effects, LLM calls, and orchestration.
- **Handlers**: Standard CRUD + cursor pagination. Keep handlers thin; they should extract request data and call services.
- **generic_crud**: `find_all`, `find_after`, `find_by_id`, `insert`, `update`, `delete` — imported from `rustwing::prelude::*`.

### Nullable PATCH fields

Use normal `Option<T>` update fields when `None` means "do not change". For nullable columns that clients must be able to clear, use `Nullable<T>` from `rustwing::prelude::*`:

```rust
#[derive(Deserialize)]
pub struct UpdateTicket {
    #[serde(default)]
    pub assigned_member_id: Nullable<Uuid>,
}
```

Interpret it as:
- `Nullable::Missing` — do not update this column
- `Nullable::Null` — write SQL `NULL`
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

This is intentionally opt-in because not every optional database column needs clear-via-PATCH semantics.

### Worker pattern

The generated `worker` binary is executable, not just a placeholder. It loads `.env`, configures tracing, connects to Postgres, builds the configured LLM client, creates `WorkerState { db, llm }`, and runs `process_pending_jobs()` on an interval controlled by `WORKER_TICK_SECONDS`.

Put polling, queues, AI enrichment, and other background workflows in worker services/functions called from `process_pending_jobs()`.

### Migrations

- SQL migration files in `api/migrations/` — auto-run on server startup via `sqlx::migrate!()`.
- Trigger function (`00000000000000_create_trigger_function.sql`) is shared by all tables.
- Name format: `<version>_<description>.sql` (version is zero-padded 14-digit number).
- Trigger `set_timestamp` updates `updated_at` on every row UPDATE.

### Error handling

- `AppError` wraps `CoreError` (Database, NotFound, Unauthorized, Internal) and `ValidationErrors`.
- Database constraint violations (`23505` = unique, `23503` = foreign key) map to `409 Conflict`.
- `NotFound` maps to `404`; `Unauthorized` maps to `401`.
- All other errors map to `500`.

## Common commands

```bash
cargo check                              # Verify compilation
cargo run --bin api                      # Start the API server (migrations auto-run)
cargo run --bin worker                   # Start the background worker tick loop
rustwing g --help                        # Show generator arguments and flags
rustwing g resource <name> --fields ...  # Generate a full REST resource
rustwing g resource ticket --tenant org_id --fields 'org_id:uuid:required' --fields 'subject:string:required'
rustwing g resource comment --scope ticket_id --fields 'ticket_id:uuid:required' --fields 'body:string:required'
rustwing g model <name> ...              # Generate a data-only model
```

## If adding a new resource manually

Do this only when the CLI is blocked or the required change is beyond what it can generate. Otherwise, use `rustwing g resource`.

1. Create domain model in `api/src/domain/<name>.rs` and add `pub mod` to `mod.rs`
2. Create repository in `api/src/repository/<name>_repo.rs` with `ModelName` impl
3. Create DTOs in `api/src/http/dtos/<name>_dto.rs`
4. Create service functions in `api/src/services/<name>_service.rs`
5. Create handlers in `api/src/http/handlers/<name>_routes.rs`
6. Register routes in `api/src/http/mod.rs`
7. Register OpenAPI paths/schemas/tags in `api/src/openapi.rs`
8. Create migration SQL file in `api/migrations/`

## Framework reference

- **Source & issues**: https://github.com/creativebash/rustwing
- **Framework crate**: `rustwing` (provides `AuthEngine`, `generic_crud`, `ModelName`/`Insertable`/`Updateable` traits)
- **CLI crate**: `rustwing-cli` (provides `rustwing new`, `rustwing g`, `rustwing run`)
