# Rustwing Manifesto

Rustwing is an application framework for building Rust apps with Axum.

Its value is structure, scaffolding, and conventions, not hiding SQL or replacing the Rust ecosystem.

## What Rustwing Optimizes For

Rustwing exists to help developers move from an empty directory to a production-shaped backend quickly.

It should make the common app path obvious:

- Create a project
- Add authenticated resources
- Keep business logic in services
- Keep database access in repositories
- Run migrations automatically
- Add workers when background work appears
- Use LLM integrations when they help the product

The generated code should feel like code a careful Rust developer would have written by hand.

## Principles

### Rustwing Extends Axum

Rustwing does not replace Axum. It gives Axum projects a useful application shape.

Custom routes, extractors, middleware, state, and handlers should remain ordinary Axum code.

### SQL Is Not a Leak

Rustwing is SQL-native.

Repositories use SQLx and explicit SQL. The generator removes repetitive boilerplate, but it should not invent an ORM or hide database behavior behind a private query language.

When a query becomes custom, developers should write normal SQLx.

### Generated Code Belongs to the App

Rustwing generates real Rust files that the application owns.

Generated code should be readable, editable, and boring. No hidden runtime registry should be required to understand how a request reaches the database.

### Services Are the Extension Point

Handlers should stay thin. Services should own validation, tenant scope, side effects, LLM calls, background-job enqueueing, and business workflows.

The generated path should teach this convention from the first project.

### SaaS Is First Class, Not Mandatory

Single-tenant apps should stay simple by default.

SaaS apps should opt into tenant-scoped generation explicitly. Parent-child resources should opt into scoped generation explicitly. Scope should be visible in routes, services, repositories, and SQL.

### Batteries Included, Ejectable by Default

Auth, CRUD scaffolding, migrations, workers, errors, and LLM hooks should work out of the box.

But every battery should be ordinary Rust code that can be changed or removed without fighting the framework.

### Clear Beats Clever

Rustwing should prefer explicit files, explicit SQL, explicit routing, and explicit conventions over clever abstractions.

If the generated code is easy to inspect and easy to modify, Rustwing is doing its job.

## What Rustwing Should Not Become

Rustwing should not become:

- A new web framework that competes with Axum
- A custom ORM
- A macro-heavy system where generated behavior is hard to trace
- A platform that forces one deployment model
- A framework that hides Rust, SQLx, Tokio, or Axum from the developer

## The Promise

Rustwing should be the fastest credible path from zero to a production-shaped Axum backend.

It should help developers start clean, stay organized, and keep ownership of their code.
