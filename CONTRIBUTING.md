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

## Local development (without publishing)

You don't need to publish to crates.io to test changes. Here are several approaches:

### Use the local CLI directly

Run CLI commands from the workspace root without installing:

```bash
# Create a project
cargo run --bin rustwing -- new my_test_app

# Generate a resource (from within the project)
cargo run --bin rustwing -- g resource post --fields 'title:string:required:length(1,255)'
```

### Install the local CLI over an existing one

```bash
# Install from local source (overwrites any published version)
cargo install --path cli --force

# Now `rustwing` uses your local changes
rustwing new my_test_app
```

### Test framework changes in a generated project

When you modify the `rustwing` crate, test it in a real project by overriding the dependency:

```bash
# Use --local to auto-patch the project to use your local rustwing checkout
cargo run --bin rustwing -- new --local /path/to/rustwing/repo my_test_app
cd my_test_app
cargo check   # compiles against local rustwing
```

Or manually add a `[patch]` section to the project's workspace `Cargo.toml`:
```toml
[patch.crates-io]
rustwing = { path = "/path/to/rustwing/repo/rustwing" }
```

### Test CLI template changes

The template files in `cli/template/` are the source of truth. Edit them directly, then regenerate the embedded copy:

```bash
cd cli && cargo run --bin gen-template
```

Test your template changes:

```bash
# Verify the workspace still compiles
cargo check

# Create a test project and verify it compiles
cargo run --bin rustwing -- new test_project
cd test_project && cargo check
cd .. && rm -rf test_project
```

### Full end-to-end local test (framework + CLI)

```bash
# 1. Make changes in rustwing/ or cli/
# 2. Build the CLI
cargo build --bin rustwing

# 3. Create a test project with local framework path
./target/debug/rustwing new --local "$(pwd)/rustwing" test_e2e

# 4. Test it compiles and works
cd test_e2e && cargo check && cd ..
```

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
