use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn project_prefix() -> PathBuf {
    if Path::new("api/src/domain/mod.rs").exists() {
        PathBuf::from("api")
    } else if Path::new("src/domain/mod.rs").exists() {
        PathBuf::from(".")
    } else {
        eprintln!("❌ No src/ or api/src/ directory found. Run this from your project root.");
        std::process::exit(1);
    }
}

fn migrations_dir() -> PathBuf {
    let p = project_prefix();
    if p == PathBuf::from(".") {
        PathBuf::from("migrations")
    } else {
        p.join("migrations")
    }
}

#[derive(Debug, Clone)]
struct Field {
    name: String,
    rust_type: String,
    sql_type: String,
    is_required: bool,
    validator_hint: Option<String>,
}

impl Field {
    fn from_arg(arg: &str) -> Result<Self, String> {
        let parts: Vec<&str> = arg.splitn(4, ':').collect();

        if parts.len() < 3 {
            return Err(format!(
                "Invalid field format: '{}'. Use: name:type:required|optional[:hint]",
                arg
            ));
        }

        let name = parts[0].to_string();
        let type_str = parts[1].to_lowercase();
        let is_required = parts[2].to_lowercase() == "required";
        let validator_hint = parts.get(3).map(|s| s.to_string());

        let (rust_type, sql_type) = match type_str.as_str() {
            "string" => ("String".to_string(), "TEXT".to_string()),
            "int" | "i32" => ("i32".to_string(), "INTEGER".to_string()),
            "i64" => ("i64".to_string(), "BIGINT".to_string()),
            "float" | "f64" => ("f64".to_string(), "DECIMAL(10,2)".to_string()),
            "bool" => ("bool".to_string(), "BOOLEAN".to_string()),
            "uuid" => ("Uuid".to_string(), "UUID".to_string()),
            "datetime" => ("DateTime<Utc>".to_string(), "TIMESTAMPTZ".to_string()),
            "json" | "jsonb" => ("serde_json::Value".to_string(), "JSONB".to_string()),
            "ref" => {
                let ref_table = format!("{}s", name.trim_end_matches("_id"));
                ("Uuid".to_string(), format!("UUID REFERENCES {} (id)", ref_table))
            }
            other => {
                return Err(format!(
                    "Unknown type: '{}'. Supported: string, int, i32, i64, float, f64, bool, uuid, datetime, json, jsonb, ref",
                    other
                ));
            }
        };

        Ok(Field {
            name,
            rust_type,
            sql_type,
            is_required,
            validator_hint,
        })
    }

    fn rust_field_type(&self) -> String {
        if self.is_required {
            self.rust_type.clone()
        } else {
            format!("Option<{}>", self.rust_type)
        }
    }

    fn sql_column_def(&self) -> String {
        let null = if self.is_required { "NOT NULL" } else { "" };
        format!("    {} {} {}", self.name, self.sql_type, null)
            .trim_end()
            .to_string()
    }

    fn validator_attr(&self) -> String {
        if let Some(ref hint) = self.validator_hint {
            let hint = hint.trim().to_lowercase();
            if hint == "none" { return String::new(); }
            if let Some(inner) = hint.strip_prefix("length(").and_then(|s| s.strip_suffix(')')) {
                let nums: Vec<&str> = inner.split(',').map(str::trim).collect();
                if nums.len() == 2 {
                    return format!("#[validate(length(min = {}, max = {}))]", nums[0], nums[1]);
                }
            }
            if let Some(inner) = hint.strip_prefix("range(").and_then(|s| s.strip_suffix(')')) {
                let nums: Vec<&str> = inner.split(',').map(str::trim).collect();
                if nums.len() == 2 {
                    return format!("#[validate(range(min = {}, max = {}))]", nums[0], nums[1]);
                }
                if nums.len() == 1 {
                    return format!("#[validate(range(min = {}))]", nums[0]);
                }
            }
            if hint == "email" { return "#[validate(email)]".to_string(); }
            if hint == "url" { return "#[validate(url)]".to_string(); }
            return format!("#[validate({})]", hint);
        }
        match self.rust_type.as_str() {
            "String" => "#[validate(length(min = 1, max = 255))]".to_string(),
            "i32" | "i64" => "#[validate(range(min = 0))]".to_string(),
            "f64" => "#[validate(range(min = 0.0))]".to_string(),
            _ => String::new(),
        }
    }

    fn is_auto(&self) -> bool {
        (self.rust_type == "DateTime<Utc>") || (self.rust_type == "Uuid" && self.name == "id")
    }
}

pub fn run(gen_type: &str, name: &str, field_args: &[String]) {
    if gen_type != "model" && gen_type != "resource" {
        eprintln!("❌ Invalid type: '{}'. Must be 'model' or 'resource'.", gen_type);
        std::process::exit(1);
    }

    let model_lower = name.to_lowercase();
    let model_camel = uppercase_first(&model_lower);
    let table_name = format!("{}s", model_lower);

    let fields: Vec<Field> = field_args
        .iter()
        .map(|a| Field::from_arg(a))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|e| {
            eprintln!("❌ {}", e);
            std::process::exit(1);
        });

    println!(
        "🚀 Generating {} '{}' with {} field(s)...\n",
        gen_type,
        model_camel,
        fields.len()
    );

    let prefix = project_prefix();
    let s = |path: &str| -> String {
        prefix.join(path).to_string_lossy().to_string()
    };
    let m_dir = migrations_dir();

    create_file(
        &s(&format!("src/domain/{}.rs", model_lower)),
        &domain_template(&model_camel, &fields),
    );
    create_file(
        &s(&format!("src/repository/{}_repo.rs", model_lower)),
        &repo_template(&model_camel, &model_lower, &table_name, gen_type, &fields),
    );
    append_mod(&s("src/domain/mod.rs"), &model_lower);
    append_mod(&s("src/repository/mod.rs"), &format!("{}_repo", model_lower));

    if gen_type == "resource" {
        create_file(
            &s(&format!("src/http/dtos/{}_dto.rs", model_lower)),
            &dto_template(&model_camel, &model_lower, &fields),
        );
        create_file(
            &s(&format!("src/http/handlers/{}_routes.rs", model_lower)),
            &handler_template(&model_camel, &model_lower),
        );
        append_mod(&s("src/http/dtos/mod.rs"), &format!("{}_dto", model_lower));
        append_mod(&s("src/http/handlers/mod.rs"), &format!("{}_routes", model_lower));
        inject_router(&prefix, &model_lower, &table_name);
    }

    fs::create_dir_all(&m_dir).unwrap_or_else(|e| panic!("Failed to create migrations dir: {}", e));

    let trigger_path = m_dir.join("00000000000000_create_trigger_function.sql");
    create_file(&trigger_path.to_string_lossy(), TRIGGER_FUNCTION_SQL);

    let ts = migration_timestamp();
    let mpath = m_dir.join(format!("{}_create_{}.sql", ts, table_name));
    create_file(&mpath.to_string_lossy(), &table_migration_sql(&table_name, &fields));

    println!();
    println!("✅ Done!");
    println!();
    println!("📝 Next steps:");
    println!("   1. Run `cargo check`");
    println!("   2. Run `sqlx migrate run`");
    if gen_type == "resource" {
        println!("   3. Try: curl -X POST http://localhost:3000/{}", table_name);
    }
}

fn create_file(path: &str, content: &str) {
    if Path::new(path).exists() {
        println!("⚠️  Skipped: {} (already exists)", path);
        return;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .unwrap_or_else(|e| panic!("Failed to create {}: {}", path, e));
    file.write_all(content.as_bytes()).unwrap();
    println!("   📄 Created: {}", path);
}

fn append_mod(file_path: &str, mod_name: &str) {
    let stmt = format!("pub mod {};\n", mod_name);
    if let Ok(content) = fs::read_to_string(file_path) {
        if content.contains(&stmt) {
            return;
        }
    }
    let mut file = OpenOptions::new()
        .append(true)
        .open(file_path)
        .unwrap_or_else(|e| panic!("Failed to open {}: {}", file_path, e));
    file.write_all(stmt.as_bytes()).unwrap();
    println!("   ⚙️  Updated: {}", file_path);
}

fn inject_router(prefix: &Path, lower: &str, plural: &str) {
    let router_path = prefix.join("src/http/mod.rs");
    let content = fs::read_to_string(&router_path).expect("Failed to read http/mod.rs");

    if content.contains(&format!("handlers::{}_routes", lower)) {
        println!("⚠️  Skipped: router injection (routes already exist)");
        return;
    }

    let new_routes = format!(
        "// {Model} routes\n        \
         .route(\"/{plural}\", get(handlers::{lower}_routes::list_{lower}s).post(handlers::{lower}_routes::create_{lower}))\n        \
         .route(\"/{plural}/cursor\", get(handlers::{lower}_routes::list_{lower}s_cursor))\n        \
         .route(\"/{plural}/{{id}}\", get(handlers::{lower}_routes::get_{lower}).put(handlers::{lower}_routes::update_{lower}).delete(handlers::{lower}_routes::delete_{lower}))\n        \
         .with_state(state)",
        Model = uppercase_first(lower),
        plural = plural,
        lower = lower,
    );

    let updated = content.replace(".with_state(state)", &new_routes);
    fs::write(&router_path, updated).unwrap();
    println!("   🔀 Injected: routes into {}", router_path.display());
}

fn uppercase_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn migration_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut rem = secs;
    let ss = rem % 60;
    rem /= 60;
    let mm = rem % 60;
    rem /= 60;
    let hh = rem % 24;
    rem /= 24;
    let (y, mo, d) = days_to_ymd(rem as u32);

    format!("{:04}{:02}{:02}{:02}{:02}{:02}", y, mo, d, hh, mm, ss)
}

fn days_to_ymd(mut days: u32) -> (u32, u32, u32) {
    let year = 1970u32;
    let mut y = year;
    loop {
        let in_year = if is_leap(y) { 366 } else { 365 };
        if days < in_year { break; }
        days -= in_year;
        y += 1;
    }
    let ml = [
        31u32,
        if is_leap(y) { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mut month = 1u32;
    for &len in &ml {
        if days < len { break; }
        days -= len;
        month += 1;
    }
    (y, month, days + 1)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn table_migration_sql(table: &str, fields: &[Field]) -> String {
    let col_defs: String = fields
        .iter()
        .filter(|f| !f.is_auto())
        .map(|f| format!("{},\n", f.sql_column_def()))
        .collect();

    format!(
        r#"-- Create {table} table
CREATE TABLE {table} (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
{col_defs}    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON {table}
FOR EACH ROW EXECUTE PROCEDURE trigger_set_timestamp();
"#,
        table = table,
        col_defs = col_defs,
    )
}

const TRIGGER_FUNCTION_SQL: &str = r#"CREATE OR REPLACE FUNCTION trigger_set_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
"#;

fn domain_template(model: &str, fields: &[Field]) -> String {
    let field_defs: String = fields
        .iter()
        .map(|f| format!("    pub {}: {},\n", f.name, f.rust_field_type()))
        .collect();

    format!(
        r#"use chrono::{{DateTime, Utc}};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow, Clone)]
pub struct {Model} {{
    pub id: Uuid,
{fields}    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}}
"#,
        Model = model,
        fields = field_defs,
    )
}

fn dto_template(model: &str, lower: &str, fields: &[Field]) -> String {
    let create_fields: String = fields
        .iter()
        .filter(|f| !f.is_auto())
        .map(|f| {
            let attr = f.validator_attr();
            if attr.is_empty() {
                format!("    pub {}: {},\n", f.name, f.rust_type)
            } else {
                format!("    {}\n    pub {}: {},\n", attr, f.name, f.rust_type)
            }
        })
        .collect();

    let update_fields: String = fields
        .iter()
        .filter(|f| !f.is_auto())
        .map(|f| format!("    pub {}: Option<{}>,\n", f.name, f.rust_type))
        .collect();

    let response_fields: String = fields
        .iter()
        .map(|f| format!("    pub {}: {},\n", f.name, f.rust_field_type()))
        .collect();

    let from_fields: String = fields
        .iter()
        .map(|f| format!("            {}: model.{},\n", f.name, f.name))
        .collect();

    format!(
        r#"use chrono::{{DateTime, Utc}};
use serde::{{Deserialize, Serialize}};
use uuid::Uuid;
use validator::Validate;
use crate::domain::{lower}::{Model};

#[derive(Deserialize, Validate)]
pub struct Create{Model} {{
{create_fields}}}

#[derive(Deserialize)]
pub struct Update{Model} {{
{update_fields}}}

#[derive(Serialize, Clone)]
pub struct {Model}Response {{
    pub id: Uuid,
{response_fields}    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}}

impl From<{Model}> for {Model}Response {{
    fn from(model: {Model}) -> Self {{
        Self {{
            id: model.id,
{from_fields}            created_at: model.created_at,
            updated_at: model.updated_at,
        }}
    }}
}}
"#,
        Model = model,
        lower = lower,
        create_fields = create_fields,
        update_fields = update_fields,
        response_fields = response_fields,
        from_fields = from_fields,
    )
}

fn repo_template(model: &str, lower: &str, table: &str, gen_type: &str, fields: &[Field]) -> String {
    let imports = if gen_type == "resource" {
        format!(
            "use crate::{{http::dtos::{lower}_dto::{{Create{Model}, Update{Model}}}, domain::{lower}::{Model}}};",
            lower = lower, Model = model
        )
    } else {
        format!(
            "use crate::domain::{lower}::{Model};",
            lower = lower, Model = model
        )
    };

    let resource_impls = if gen_type == "resource" {
        let data_fields: Vec<&Field> = fields.iter().filter(|f| !f.is_auto()).collect();

        let columns: String = data_fields
            .iter()
            .map(|f| format!("\"{}\"", f.name))
            .collect::<Vec<_>>()
            .join(", ");

        let insert_fields: String = data_fields
            .iter()
            .map(|f| format!("    pub {}: {},\n", f.name, f.rust_type))
            .collect();

        let update_fields: String = data_fields
            .iter()
            .map(|f| format!("    pub {}: Option<{}>,\n", f.name, f.rust_type))
            .collect();

        let from_insert_fields: String = data_fields
            .iter()
            .map(|f| format!("            {}: dto.{},\n", f.name, f.name))
            .collect();

        let from_update_fields: String = data_fields
            .iter()
            .map(|f| format!("            {}: dto.{},\n", f.name, f.name))
            .collect();

        let bind_values: String = data_fields
            .iter()
            .map(|f| format!("        separated.push_bind(&self.{});\n", f.name))
            .collect();

        let bind_updates: String = data_fields
            .iter()
            .map(|f| format!(
                "        if let Some(ref v) = self.{name} {{ separated.push(\"{name} = \").push_bind_unseparated(v); has_updates = true; }}\n",
                name = f.name
            ))
            .collect();

        format!(
            r#"

pub struct Insert{Model} {{
{insert_fields}}}

impl From<Create{Model}> for Insert{Model} {{
    fn from(dto: Create{Model}) -> Self {{
        Self {{
{from_insert_fields}        }}
    }}
}}

impl Insertable for Insert{Model} {{
    fn columns() -> Vec<&'static str> {{
        vec![{columns}]
    }}
    fn bind_values<'a>(&'a self, query: &mut QueryBuilder<'a, Postgres>) {{
        let mut separated = query.separated(", ");
{bind_values}    }}
}}

pub struct {Model}Update {{
{update_fields}}}

impl From<Update{Model}> for {Model}Update {{
    fn from(dto: Update{Model}) -> Self {{
        Self {{
{from_update_fields}        }}
    }}
}}

impl Updateable for {Model}Update {{
    fn bind_updates<'a>(&'a self, query: &mut QueryBuilder<'a, Postgres>) -> UpdateResult {{
        let mut separated = query.separated(", ");
        let mut has_updates = false;

{bind_updates}        if has_updates {{ UpdateResult::HasUpdates }} else {{ UpdateResult::NoChanges }}
    }}
}}"#,
            Model = model,
            insert_fields = insert_fields,
            update_fields = update_fields,
            columns = columns,
            bind_values = bind_values,
            bind_updates = bind_updates,
            from_insert_fields = from_insert_fields,
            from_update_fields = from_update_fields,
        )
    } else {
        String::new()
    };

    let uuid_import = if gen_type == "resource" {
        "use uuid::Uuid;\n"
    } else {
        ""
    };

    format!(
        r#"use sqlx::{{Postgres, QueryBuilder}};
use rustwing::prelude::*;
{uuid_import}{imports}

impl ModelName for {Model} {{
    fn table_name() -> &'static str {{ "{table}" }}
}}{resource_impls}
"#,
        Model = model,
        table = table,
        imports = imports,
        resource_impls = resource_impls,
        uuid_import = uuid_import,
    )
}

fn handler_template(model: &str, lower: &str) -> String {
    format!(
        r#"use axum::{{extract::{{Path, Query, State}}, http::StatusCode, Json}};
use uuid::Uuid;
use validator::Validate;

use crate::{{
    http::dtos::{lower}_dto::{{Create{Model}, Update{Model}, {Model}Response}},
    http::extractors::AuthUser,
    http::handlers::user_routes::{{Pagination, CursorPagination}},
    domain::{lower}::{Model},
    error::AppError,
    repository::{lower}_repo::{{Insert{Model}, {Model}Update}},
    state::AppState,
}};
use rustwing::prelude::*;

pub async fn list_{lower}s(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<{Model}Response>>, AppError> {{
    let items = generic_crud::find_all::<{Model}>(&state.db, p.limit.unwrap_or(10), p.offset.unwrap_or(0)).await?;
    Ok(Json(items.into_iter().map({Model}Response::from).collect()))
}}

pub async fn list_{lower}s_cursor(
    auth: AuthUser,
    State(state): State<AppState>,
    Query(p): Query<CursorPagination>,
) -> Result<Json<Vec<{Model}Response>>, AppError> {{
    let after = p.after.unwrap_or_else(Uuid::nil);
    let items = generic_crud::find_after::<{Model}>(&state.db, after, p.limit.unwrap_or(10)).await?;
    Ok(Json(items.into_iter().map({Model}Response::from).collect()))
}}

pub async fn get_{lower}(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<{Model}Response>, AppError> {{
    let item = generic_crud::find_by_id::<{Model}>(&state.db, id).await?;
    Ok(Json({Model}Response::from(item)))
}}

pub async fn create_{lower}(
    auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<Create{Model}>,
) -> Result<(StatusCode, Json<{Model}Response>), AppError> {{
    payload.validate()?;
    let insert = Insert{Model}::from(payload);
    let item = generic_crud::insert::<{Model}, Insert{Model}>(&state.db, &insert).await?;
    Ok((StatusCode::CREATED, Json({Model}Response::from(item))))
}}

pub async fn update_{lower}(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<Update{Model}>,
) -> Result<Json<{Model}Response>, AppError> {{
    let update = {Model}Update::from(payload);
    let item = generic_crud::update::<{Model}, {Model}Update>(&state.db, id, &update).await?;
    Ok(Json({Model}Response::from(item)))
}}

pub async fn delete_{lower}(
    auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {{
    generic_crud::delete::<{Model}>(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}}
"#,
        Model = model,
        lower = lower,
    )
}
