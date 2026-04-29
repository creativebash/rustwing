# Contributing

## Code of conduct

This project is committed to providing a welcoming, inclusive environment. Be respectful, assume good faith, and help us build something great.

## Setup

```bash
git clone git@github.com:creativebash/rustwing.git
cd rustwing
cargo build
```

## Workspace layout

```
rustwing/            # Framework library — published to crates.io
├── src/
│   ├── error.rs            # CoreError (Database, NotFound, Internal)
│   ├── repository/         # Generic CRUD + traits (ModelName, Insertable, Updateable)
│   └── infrastructure/     # Auth (Argon2 + JWT), LLM (DeepSeek/Stub)
api/                 # Development sample app — proves the framework compiles
cli/                 # CLI binary — published as rustwing-cli
├── src/
│   ├── main.rs             # Subcommands: new, generate
│   ├── new.rs              # Copies template/ → project directory
│   └── generate.rs         # Scaffolds resources: domain, DTOs, handlers, migration
└── template/        # What `rustwing new` generates (real compilable Rust files)
    ├── Cargo.toml          # Uses {{project_name}} placeholder
    ├── api/src/            # Full API skeleton with auth + user CRUD
    ├── worker/src/         # Background worker skeleton
    └── frontend/           # Placeholder directory
```

## Making changes

### 1. Change the framework (`rustwing/`)

Make your changes in `rustwing/src/`. Run `cargo check` to verify they compile.

### 2. Update the template (`cli/template/`)

If your framework change affects generated code (new trait, changed signatures, new feature), update the corresponding files in `cli/template/`. These are real Rust files — `cargo check` will catch breakage.

The template uses `{{project_name}}` as a placeholder that gets replaced when users run `rustwing new my_app`.

### 3. Update the dev crate (`api/`)

The `api/` crate is the dogfooding reference. It should stay in sync with the template. When you add a feature to the framework or template, mirror it here.

### 4. Update the scaffold generator (`cli/src/generate.rs`)

If you add new framework features that should be scaffoldable (new resource types, new field types), update `generate.rs`. This handles `rustwing g resource ...`.

## Testing

```bash
# Full workspace
cargo check

# Test the new command end-to-end
cargo run --bin rustwing -- new test_project
cd test_project && cargo check
cd .. && rm -rf test_project
```

For database-dependent testing, point `DATABASE_URL` at a local Postgres instance.

## Pull requests

- Keep changes focused. One PR = one concern.
- Update `cli/template/` if you change the framework API.
- Update `cli/src/generate.rs` if you add scaffoldable features.
- Run `cargo check` before submitting.
- The template pins `rustwing = "0.1"` in `api/Cargo.toml` — if you bump the framework version, update it there too.

## Publishing (maintainers only)

Publishing order matters — the framework must be on crates.io before the CLI:

```bash
# Verify the template is bundled
cargo package -p rustwing-cli --list --no-verify
# Should include template/**/* files

# Publish framework first
cargo publish -p rustwing

# Then CLI
cargo publish -p rustwing-cli
```

## Issues

- **Bug reports** — include the output of `rustwing --version` and steps to reproduce
- **Feature requests** — explain the use case, not just the desired solution
- **Questions** — open a discussion, not an issue

## License

MIT
