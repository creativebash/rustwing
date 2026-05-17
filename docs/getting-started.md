# Getting started

## Install

```bash
cargo install rustwing-cli
```

## Create a project

```bash
rustwing new my_saas
cd my_saas
```

This generates:

```
my_saas/
├── Cargo.toml              # workspace root
├── .env.example            # environment template
├── api/                    # web server
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── domain/user.rs  # User model
│   │   ├── http/           # routes, handlers, DTOs
│   │   ├── repository/     # SQLx-native database access
│   │   └── services/       # business logic and orchestration
│   └── migrations/         # auto-run on startup
├── worker/                 # DB/LLM-backed worker tick loop
└── frontend/               # your frontend (BYO)
```

## Configure

```bash
cp .env.example .env
# Edit DATABASE_URL in .env to point to your Postgres database
```

## Run

```bash
rustwing run
```

Or directly with cargo:

```bash
cargo run --bin api
```

The server:
1. Connects to Postgres
2. Runs pending migrations (creates tables automatically)
3. Starts listening on `http://0.0.0.0:3000`

## Test the API

```bash
# Register a user (returns a JWT token + user info)
curl -s -X POST http://localhost:3000/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"demo","email":"demo@test.com","password":"password123"}' | jq .
# The token in the response can be used immediately for authenticated requests.
# If the registration token does not work (e.g., "Invalid or expired token"),
# log in instead — both return the same type of token:

# Login to get a fresh token
curl -s -X POST http://localhost:3000/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"demo@test.com","password":"password123"}' | jq .

# Extract the token using jq
TOKEN=$(curl -s -X POST http://localhost:3000/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"demo@test.com","password":"password123"}' | jq -r '.token')

# List users (authenticated)
curl http://localhost:3000/users/cursor \
  -H "Authorization: Bearer $TOKEN"

# Get user by ID
curl http://localhost:3000/users/<id> \
  -H "Authorization: Bearer $TOKEN"
```

## Generate a resource

```bash
rustwing g resource post \
  --fields 'title:string:required:length(1,255)' \
  --fields 'body:string:required'
```

This creates:
- Domain model (`Post`)
- Create/Update DTOs with validation
- Service module for validation, pagination limits, and business logic
- SQLx-native repository glue and explicit CRUD behavior
- Route handlers with offset and cursor pagination
- Router registration
- Database migration

```bash
# After generating, create a post (authenticated)
curl -X POST http://localhost:3000/posts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"title":"Hello World","body":"My first post!"}'
```

## Generate a scoped resource

For SaaS data, opt into tenant scope explicitly:

```bash
rustwing g resource ticket \
  --tenant organization_id \
  --fields 'organization_id:uuid:required' \
  --fields 'subject:string:required:length(1,255)' \
  --fields 'assigned_member_id:uuid:optional'
```

This generates routes like `/organizations/{organization_id}/tickets`. The tenant ID comes from the path, so create/update request bodies do not include `organization_id`.

Scopes also work for parent-child resources:

```bash
rustwing g resource comment \
  --scope ticket_id \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

This generates routes like `/tickets/{ticket_id}/comments`. Combine scopes when a resource needs both tenant and parent boundaries:

```bash
rustwing g resource note \
  --tenant organization_id \
  --scope ticket_id \
  --fields 'organization_id:uuid:required' \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

## Run the worker

The generated worker connects to the same database, builds the configured LLM client, and runs a tick loop:

```bash
cargo run --bin worker
```

Set `WORKER_TICK_SECONDS` to change the polling interval.

## Next steps

- [CLI reference](cli-reference.md) — all `rustwing` commands
- [Architecture guide](architecture.md) — how the framework works
- [Configuration reference](configuration.md) — all environment variables
