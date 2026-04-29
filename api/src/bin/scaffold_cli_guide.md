Here is a complete guide to using your built-in CLI scaffolding tool, followed
by the solution for the authentication error you're currently facing.

Part 1: CLI Scaffolding Guide

Your project includes a powerful custom scaffolding tool located in
template/src/bin/scaffold.rs. It generates Rust code (Domain, DTOs,
Repositories, Handlers) and SQL migrations automatically so you don't have to
write boilerplate.

1. Basic Syntax

Run the tool from the root of your template directory using cargo run --bin
scaffold:

cargo run --bin scaffold -- <type> <model_name> [--fields ...]

- <type>: Can be either:
  - resource: Generates everything (Domain, Repo, DTOs, Axum Handlers, Route
    injections, and SQL Migration). Use this for standard API endpoints.
  - model: Generates only the Domain, Repository, and SQL Migration (No API
    routes or DTOs). Use this for internal tables.
- <model_name>: The singular name of your resource (e.g., product, post,
  comment).

2. Defining Fields

Add as many --fields arguments as you need. The syntax is strictly formatted as:
name:type:required|optional[:validator_hint]

Supported Types:

- string, int, i32, i64, float, f64, bool, uuid, datetime, json, jsonb

Validator Hints (Optional):

- length(min,max) (e.g., length(1,255))
- range(min) or range(min,max) (e.g., range(0))
- email, url, none

3. Examples

Example A: Generate a Full API Resource Let's generate a Product resource with
validation:

cargo run --bin scaffold -- resource product \
 --fields title:string:required:length(1,255) \
 --fields description:string:optional:none \
 --fields price:f64:required:range(0.0) \
 --fields in_stock:bool:required:none

Example B: Generate a Database-Only Model Let's generate a Subscription model
that our internal services will use (no HTTP API exposed):

cargo run --bin scaffold -- model subscription \
 --fields user_id:uuid:required:none \
 --fields plan_name:string:required \
 --fields active_until:datetime:required

4. Post-Scaffolding Steps

After running the command:

1.  Format your code: cargo fmt
2.  Ensure it compiles: cargo check
3.  Run the newly generated database migration:
    sqlx migrate run
