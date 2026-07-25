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
2. Runs pending migrations (and fails if applied migration history has drifted)
3. Starts listening on `http://0.0.0.0:3000`
4. Serves OpenAPI docs at `http://localhost:3000/docs` and `http://localhost:3000/redoc`

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

# Read the authenticated account
curl http://localhost:3000/users/me \
  -H "Authorization: Bearer $TOKEN"
```

The starter account endpoints are self-only. A valid token does not grant
access to other accounts; application-specific admin or directory endpoints
should be added with an explicit authorization policy.

## Generate a resource

```bash
rustwing g resource post \
  --fields 'title:string:required:length(1,255)' \
  --fields 'body:text:required'
```

This creates:
- Domain model (`Post`)
- Create/Update DTOs with validation
- Service module for validation, pagination limits, and business logic
- SQLx-native repository glue and explicit CRUD behavior
- Route handlers with offset and cursor pagination
- Router registration
- OpenAPI schemas and route metadata
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
  --tenant org_id \
  --fields 'org_id:uuid:required' \
  --fields 'subject:string:required:length(1,255)' \
  --fields 'assigned_member_id:uuid:optional'
```

This generates routes like `/orgs/{org_id}/tickets`. The tenant ID comes from the path, so create/update request bodies do not include `org_id`.

The generated route and repository remain scoped to that ID, but your service
must also verify that the authenticated user belongs to the organization.

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
  --tenant org_id \
  --scope ticket_id \
  --fields 'org_id:uuid:required' \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

## Export OpenAPI and generate a frontend client

```bash
rustwing g openapi
rustwing g client typescript
```

This writes `openapi/openapi.json` and a typed fetch client in `frontend/generated/`.

Use `rustwing g openapi --check` in CI to fail when the checked-in contract is out of date.

## Run the worker

The generated worker connects to the same database, builds the configured LLM client, and runs a tick loop:

```bash
cargo run --bin worker
```

Set `WORKER_TICK_SECONDS` to change the polling interval.

## Next steps

- [CLI reference](cli-reference.md) — all `rustwing` commands
- [OpenAPI and TypeScript client](openapi.md) — docs, contract export, and generated frontend client
- [Architecture guide](architecture.md) — how the framework works
- [Configuration reference](configuration.md) — all environment variables
