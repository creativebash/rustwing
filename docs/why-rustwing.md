# Why Rustwing

Rustwing is an application framework for building Rust backends on top of
Axum and SQLx.

It exists for developers who want the productivity of a conventional backend
framework without giving up ordinary Axum handlers, explicit SQLx database
access, or ownership of the generated code.

```txt
Axum handles HTTP.
SQLx keeps database access explicit.
Rustwing handles the application structure around them.
```

## The gap

Rust is a strong language for serious backend systems, but starting a real
product in Rust can still involve more repeated setup than it should.

Most API and SaaS backends need the same early shape:

- project workspace
- HTTP server
- database pool
- migrations
- users and auth
- request and response DTOs
- validation
- services
- repositories
- CRUD endpoints
- error mapping
- API documentation
- frontend API contracts
- background workers

None of this is unusual. The cost is that every new app has to assemble and
maintain this structure before product work can move quickly.

Rustwing closes that gap by generating a production-shaped Axum application
from the start.

## The core idea

Rustwing's core idea is:

> The fastest credible Rust backend is generated, explicit, service-first Axum
> code with SQLx underneath.

"Fast" matters because developers need momentum when building a first version.

"Credible" matters because backend code eventually has to be debugged,
customized, reviewed, extended, and operated.

Rustwing is designed to help in the first five minutes without becoming a
problem in the next five months.

## What Rustwing adds to Axum

Axum is intentionally focused. It gives Rust developers an excellent HTTP
foundation, but it does not try to define a whole application architecture.

Rustwing adds the repeated app layer around Axum:

- auth with Argon2 password hashing and JWT tokens
- service-first CRUD scaffolding
- SQLx-native repositories
- database migrations
- validation DTOs
- thin route handlers
- consistent error mapping
- scoped SaaS and parent-child resources
- OpenAPI JSON, Swagger UI, and ReDoc
- TypeScript client generation
- a worker binary with database and LLM wiring
- pluggable LLM providers with a local stub option

A Rustwing app is still an Axum app. Custom routes, extractors, middleware,
state, and handlers remain ordinary Axum code.

## Full resource slices

`rustwing g resource` generates a full REST resource, not just a model stub.

```bash
rustwing g resource product \
  --fields 'title:string:required:length(1,255)' \
  --fields 'price:f64:required:range(0.0,9999.0)'
```

That command creates the resource's vertical slice:

- domain model
- create, update, and response DTOs
- validation annotations
- service functions
- SQLx repository implementation
- Axum handlers
- route registration
- OpenAPI metadata
- database migration

The generated code follows the same path a hand-written app should follow:

```txt
HTTP route -> handler -> service -> repository -> SQLx -> database
```

Handlers extract request data. Services own validation, orchestration,
business rules, tenant scope, side effects, and LLM calls. Repositories own
database access.

## Generated code belongs to the app

Rustwing generates normal Rust files inside the application.

The generated code is meant to be:

- readable
- typed
- editable
- easy to replace
- easy to extend
- easy to debug

There is no hidden runtime registry required to understand how a request
reaches the database. Developers can open the files, follow the route, and
change the implementation when the product needs custom behavior.

That is a central Rustwing principle:

> Generated code belongs to the app.

## SQL stays explicit

Rustwing uses SQLx because SQL should stay visible.

The framework can provide generic CRUD helpers because generated repository
code describes table names, insert columns, and update bindings. But Rustwing
does not introduce an ORM or a private query language.

When a query becomes custom, the answer is normal SQLx.

This keeps the productivity gain focused on repeated boilerplate while keeping
database behavior inspectable and familiar to Rust backend developers.

## Scoped resources are first class

Many applications need data boundaries:

- organizations have tickets
- tickets have comments
- projects have tasks
- teams have members
- customers have invoices

Rustwing keeps those boundaries route-driven and explicit.

```bash
rustwing g resource ticket \
  --tenant org_id \
  --fields 'org_id:uuid:required' \
  --fields 'subject:string:required:length(1,255)'
```

This generates routes like:

```txt
/orgs/{org_id}/tickets
/orgs/{org_id}/tickets/{ticket_id}
```

The scope ID comes from the path, not the request body. Services receive it
explicitly. Repository helpers include it in SQL filters. OpenAPI documents it
as a path parameter.

That explicit scope prevents accidental unscoped queries, but it does not
replace authorization. Applications must verify that the current actor belongs
to or may access the requested organization, project, or parent resource.

For non-tenant parent-child routes, use `--scope`:

```bash
rustwing g resource comment \
  --scope ticket_id \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

Single-tenant CRUD stays the default. Scoping is opt-in because the boundary
should be visible when it exists.

## API-first by default

Generated apps expose API docs from day one:

```txt
/openapi.json
/docs
/redoc
```

That makes the generated backend easy to inspect and test immediately.

Rustwing can also export the OpenAPI contract and generate a TypeScript client:

```bash
rustwing g openapi
rustwing g client typescript
```

This keeps frontend and backend code aligned around the same API contract.

## Workers and LLM hooks

Many modern backends combine normal product infrastructure with background
work and AI features.

Rustwing includes a worker scaffold with database access and configurable LLM
providers so applications have a clear place for asynchronous workflows,
scheduled work, and AI-backed tasks.

The AI integration is intentionally part of the application structure. It does
not replace the normal backend layers: auth, resources, services,
repositories, migrations, API docs, and clients still matter.

## When Rustwing fits

Rustwing is a good fit when you want:

- an Axum-based application, not a different web framework
- SQLx-native database access, not an ORM
- generated CRUD that is readable and owned by the app
- service and repository boundaries from the start
- API docs and a frontend contract from day one
- explicit tenant or parent-child scoped resources
- a backend shape that humans and coding agents can navigate consistently

## When Rustwing may not fit

Rustwing may not be the right tool if you want:

- a runtime CMS
- a no-code backend
- a full ORM abstraction
- a framework that hides Axum
- a macro-heavy system where behavior is difficult to trace
- total control over every file from the first line of the project

Those tradeoffs are deliberate. Rustwing optimizes for clear, generated,
service-first Rust applications that remain easy to inspect and change.

## Try it

```bash
cargo install rustwing-cli
rustwing new my_app
cd my_app
rustwing run
```

Then open:

```txt
http://localhost:3000/docs
```

Generate a resource:

```bash
rustwing g resource post \
  --fields 'title:string:required:length(1,255)' \
  --fields 'body:text:required'
```

The result is a working API slice with model, DTOs, service, repository,
handlers, routes, migration, and OpenAPI metadata.

## Related docs

- [Getting started](getting-started.md)
- [CLI reference](cli-reference.md)
- [Architecture guide](architecture.md)
- [OpenAPI and TypeScript client](openapi.md)
- [Configuration reference](configuration.md)
- [Manifesto](../MANIFESTO.md)
