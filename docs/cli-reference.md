# CLI reference

## `rustwing new <name>`

Creates a new Rustwing project.

```bash
rustwing new my_app
```

Generates a complete workspace with an API server, background worker skeleton, and frontend directory.

## `rustwing generate` (alias: `g`)

Generates new code within an existing project.

### `rustwing generate resource <name>`

Generates a full REST resource: model, DTOs, repository glue, handlers, router entries, and migration.

```bash
rustwing g resource product \
  --fields 'title:string:required:length(1,255)' \
  --fields 'price:f64:required:range(0.0,9999.0)' \
  --fields 'inventory:i32:required:range(0,10000)' \
  --fields 'description:string:optional:none'
```

### `rustwing generate model <name>`

Generates a domain model, repository trait impl, and migration — no HTTP layer.

```bash
rustwing g model tag name:string
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

### Validators

| Hint | Generates |
|---|---|
| `length(1,255)` | `#[validate(length(min = 1, max = 255))]` |
| `range(0.0,9999.0)` | `#[validate(range(min = 0.0, max = 9999.0))]` |
| `email` | `#[validate(email)]` |
| `url` | `#[validate(url)]` |
| `none` | No validator |
| omitted | Type-based default (`String` → length, numbers → range) |
