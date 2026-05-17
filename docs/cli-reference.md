# CLI reference

## `rustwing new <name>`

Creates a new Rustwing project.

```bash
rustwing new my_app
```

Generates a complete workspace with an API server, a DB/LLM-backed worker tick loop, and a frontend directory.

## `rustwing run`

Runs the API server (`cargo run --bin api`).

```bash
rustwing run
```

Must be run from the project root.

## `rustwing generate` (alias: `g`)

Generates new code within an existing project.

### `rustwing generate resource <name>`

Generates a full REST resource: model, DTOs, service functions, SQLx-native repository glue, handlers, router entries, and migration.

```bash
rustwing g resource product \
  --fields 'title:string:required:length(1,255)' \
  --fields 'price:f64:required:range(0.0,9999.0)' \
  --fields 'inventory:i32:required:range(0,10000)' \
  --fields 'description:string:optional:none'
```

Generated resource handlers call a generated service module first. The service owns validation, pagination normalization, and business logic extension points; repositories stay focused on database access.

### `--scope <field>`

Use `--scope` for resources that are owned by or nested under another record.

```bash
rustwing g resource comment \
  --scope ticket_id \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

This generates nested routes like `/tickets/{ticket_id}/comments`, create/update DTOs that do not accept `ticket_id` from the body, services that receive `ticket_id` from the route, and scoped SQLx repository helpers.

Scopes can be repeated:

```bash
rustwing g resource note \
  --tenant org_id \
  --scope ticket_id \
  --fields 'org_id:uuid:required' \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

This generates routes like `/orgs/{org_id}/tickets/{ticket_id}/notes` and helpers such as `find_by_org_id_and_ticket_id`.

### `--tenant <field>`

Use `--tenant` as a clearer alias for a SaaS tenant scope. It behaves like a first `--scope` value.

```bash
rustwing g resource ticket \
  --tenant org_id \
  --fields 'org_id:uuid:required' \
  --fields 'subject:string:required:length(1,255)' \
  --fields 'assigned_member_id:uuid:optional'
```

Normal single-tenant CRUD remains the default. Scoped mode is opt-in and each scope field must be present in `--fields`.

Scoped mode generates:
- Nested routes like `/orgs/{org_id}/tickets`
- Create/update DTOs that do not accept scope fields from the request body
- Service functions that receive scope IDs from the route
- Scoped repository helpers such as `find_by_org_id`, `find_by_org_id_and_id`, `update_by_org_id_and_id`, and `delete_by_org_id_and_id`

### `rustwing generate model <name>`

Generates a domain model, repository trait impl, and migration — no HTTP layer.

```bash
rustwing g model tag --fields 'name:string:required'
```

### Field types

| Type | Rust type | SQL type |
|---|---|---|
| `string` | `String` | `TEXT` |
| `int` / `i32` | `i32` | `INTEGER` |
| `i64` | `i64` | `BIGINT` |
| `float` / `f64` | `f64` | `DECIMAL(10,2)` |
| `bool` | `bool` | `BOOLEAN` |
| `uuid` | `Uuid` | `UUID` |
| `datetime` | `DateTime<Utc>` | `TIMESTAMPTZ` |
| `json` / `jsonb` | `serde_json::Value` | `JSONB` |
| `ref` | `Uuid` | `UUID REFERENCES <table> (id)` |

### Field format

```
name:type:required|optional[:validator]
```

The `ref` type auto-derives the foreign table name from the field name (`user_id` → `users`).

For nullable PATCH semantics, use `rustwing::prelude::Nullable<T>` manually in update DTOs when a field needs three states:
- missing field = do not change
- JSON `null` = set the database column to `NULL`
- value = set the database column to that value

Add `#[serde(default)]` to the DTO field so missing input maps to `Nullable::Missing`.

### Validators

| Hint | Generates |
|---|---|
| `length(1,255)` | `#[validate(length(min = 1, max = 255))]` |
| `range(0.0,9999.0)` | `#[validate(range(min = 0.0, max = 9999.0))]` |
| `email` | `#[validate(email)]` |
| `url` | `#[validate(url)]` |
| `none` | No validator |
| omitted | Type-based default (`String` → length, numbers → range) |
