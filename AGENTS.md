# Rustwing repository guide for contributors and coding agents

This file is for people and agents working on the Rustwing repository itself.
It is not the same as `cli/template/AGENTS.md`, which is copied into apps
created by `rustwing new`.

## First principles

- Rustwing is an application framework built on Axum and SQLx. Do not hide Axum,
  replace SQLx, or introduce ORM-style abstractions.
- Keep generated apps service-first: handlers extract request data, services own
  validation and orchestration, repositories own SQLx database access.
- Prefer improving the CLI generator and templates over hand-maintaining repeated
  boilerplate in multiple places.
- Keep changes focused. If a change affects generated apps, update the template,
  regenerate embedded template data, and smoke test the generated project.
- Do not edit unrelated files or undo user changes in a dirty worktree.

## Repository map

```text
rustwing/                  Framework library crate
  src/error.rs             CoreError and framework-level errors
  src/patch.rs             Nullable<T> PATCH helper
  src/repository/          generic_crud plus ModelName/Insertable/Updateable
  src/infrastructure/      auth and LLM integrations

cli/                       rustwing-cli crate
  src/main.rs              CLI entrypoint: new, run, generate/g
  src/new.rs               Copies embedded template files into new projects
  src/generate.rs          Resource/model generator used inside projects
  src/template_data.rs     Generated embedded copy of cli/template/
  template/                Source of truth for rustwing new output
```

## Change workflow

1. Framework behavior belongs in `rustwing/src/`.
2. CLI command behavior belongs in `cli/src/main.rs`, `cli/src/new.rs`, or
   `cli/src/generate.rs`.
3. Files created by `rustwing new` belong in `cli/template/`.
4. After editing `cli/template/`, run:

```bash
cd cli && cargo run --bin gen-template
```

This regenerates `cli/src/template_data.rs`. Do not edit
`cli/src/template_data.rs` by hand.

## Generator rules

- `rustwing g resource` should produce a full REST resource: domain model,
  DTOs, service module, repository impl/helpers, route handlers, router
  registration, and migration.
- `rustwing g model` should produce data-only structure: domain model,
  repository impl, and migration.
- Keep generated handlers thin. Add extension points in services before adding
  logic directly to handlers.
- Scope fields for `--tenant` and `--scope` must stay explicit, required, and
  route-driven. Create/update bodies should not accept scope fields.
- Optional generator fields must remain optional across domain, create, insert,
  migration, and OpenAPI output. Update DTOs must retain field validators.
- `id`, `created_at`, and `updated_at` are framework-managed fields and must not
  be accepted through `--fields`.
- If a feature is a common project need, prefer adding it to the generator or
  template so future users and agents do less manual work.

## Template rules

- `cli/template/` must remain a real, compilable Rust workspace.
- The template uses `{{project_name}}` where `rustwing new <name>` should
  substitute the user's project name.
- The scaffolded app guide lives at `cli/template/AGENTS.md`. Keep it focused on
  developers building apps with Rustwing. Keep this root file focused on
  contributors changing Rustwing itself.
- When framework APIs change, update template code that consumes those APIs.
- When the framework crate version changes, check `cli/template/api/Cargo.toml`
  and the framework version string in `cli/src/main.rs`.

## Common commands

```bash
cargo check
cargo test
cargo fmt
cargo clippy
scripts/check-version-drift.sh
scripts/test-template.sh

cargo run --bin rustwing -- --help
cargo run --bin rustwing -- g --help
cargo run --bin rustwing -- new test_project --local "$(pwd)"
```

Run generator commands from inside a generated project:

```bash
../target/debug/rustwing g resource post \
  --fields 'title:string:required:length(1,255)' \
  --fields 'body:string:optional' \
  --fields 'score:f64:required:range(0.0,100.0)' \
  --fields 'published_at:datetime:optional'

../target/debug/rustwing g resource ticket \
  --tenant org_id \
  --fields 'org_id:uuid:required' \
  --fields 'subject:string:required:length(1,255)'

../target/debug/rustwing g resource comment \
  --scope ticket_id \
  --fields 'ticket_id:uuid:required' \
  --fields 'body:string:required'
```

## Smoke test checklist

Use this when touching `cli/src/generate.rs`, `cli/template/`, framework APIs
used by the template, or CLI project creation:

```bash
cargo check
cargo test
cargo run --bin rustwing -- new test_e2e --local "$(pwd)"
cd test_e2e
../target/debug/rustwing g resource post --fields 'title:string:required' --fields 'body:string:optional' --fields 'published_at:datetime:optional'
../target/debug/rustwing g resource ticket --tenant org_id --fields 'org_id:uuid:required' --fields 'subject:string:required'
../target/debug/rustwing g resource comment --scope ticket_id --fields 'ticket_id:uuid:required' --fields 'body:string:required'
cargo check
cd ..
```

Remove temporary smoke projects after inspection.

## Coding conventions

- Use `rustwing::prelude::*` in generated app code when it matches existing
  template style.
- Domain models should include `id`, `created_at`, and `updated_at` unless a
  feature intentionally requires otherwise.
- Preserve explicit SQLx behavior. Generated repositories should be readable and
  easy to debug.
- Use `Nullable<T>` only when PATCH needs three states: missing, null, and value.
  Otherwise normal `Option<T>` update fields are preferred.
- Map database and auth errors through the existing `CoreError` and `AppError`
  flow rather than inventing parallel error paths.
- Keep public docs, template comments, and generated names aligned with the CLI
  behavior users actually get.

## Publishing notes

Publishing order matters:

```bash
cargo publish -p rustwing
cargo publish -p rustwing-cli
```

Before publishing, verify the CLI package includes the generated template data
and that a generated project compiles against the intended framework version.
