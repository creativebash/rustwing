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
│   │   ├── repository/     # DB glue
│   │   └── services/       # business logic
│   └── migrations/         # auto-run on startup
├── worker/                 # background job worker
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
# Register a user
curl -X POST http://localhost:3000/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"demo","email":"demo@test.com","password":"password123"}'

# Returns a JWT token — use it for authenticated requests
TOKEN="<token-from-response>"

# List users (authenticated)
curl http://localhost:3000/users/cursor \
  -H "Authorization: Bearer $TOKEN"

# Get current user profile
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
- Repository glue (zero-SQL CRUD)
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

## Next steps

- [CLI reference](cli-reference.md) — all `rustwing` commands
- [Architecture guide](architecture.md) — how the framework works
- [Configuration reference](configuration.md) — all environment variables
