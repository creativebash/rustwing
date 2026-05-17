use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

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

fn validate_identifier(value: &str, label: &str) -> Result<(), String> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(format!("Invalid {}: value cannot be empty.", label));
    };

    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(format!(
            "Invalid {} '{}': must start with a letter or underscore.",
            label, value
        ));
    }

    if !chars.all(|c| c == '_' || c.is_ascii_alphanumeric()) {
        return Err(format!(
            "Invalid {} '{}': use only letters, numbers, and underscores.",
            label, value
        ));
    }

    const RESERVED: &[&str] = &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
        "true", "type", "unsafe", "use", "where", "while",
    ];

    if RESERVED.contains(&value) {
        return Err(format!(
            "Invalid {} '{}': reserved Rust keyword.",
            label, value
        ));
    }

    Ok(())
}

fn pluralize(value: &str) -> String {
    if value.ends_with('y')
        && !matches!(
            value.chars().rev().nth(1),
            Some('a' | 'e' | 'i' | 'o' | 'u')
        )
    {
        format!("{}ies", value.trim_end_matches('y'))
    } else if value.ends_with('s') {
        format!("{}es", value)
    } else {
        format!("{}s", value)
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
        validate_identifier(&name, "field name")?;

        let type_str = parts[1].to_lowercase();
        let is_required = match parts[2].to_lowercase().as_str() {
            "required" => true,
            "optional" => false,
            other => {
                return Err(format!(
                    "Invalid field requirement: '{}'. Use 'required' or 'optional'.",
                    other
                ));
            }
        };
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
                let ref_table = pluralize(name.trim_end_matches("_id"));
                (
                    "Uuid".to_string(),
                    format!("UUID REFERENCES {} (id)", ref_table),
                )
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
            if hint == "none" {
                return String::new();
            }
            if let Some(inner) = hint
                .strip_prefix("length(")
                .and_then(|s| s.strip_suffix(')'))
            {
                let nums: Vec<&str> = inner.split(',').map(str::trim).collect();
                if nums.len() == 2 {
                    return format!("#[validate(length(min = {}, max = {}))]", nums[0], nums[1]);
                }
            }
            if let Some(inner) = hint
                .strip_prefix("range(")
                .and_then(|s| s.strip_suffix(')'))
            {
                let nums: Vec<&str> = inner.split(',').map(str::trim).collect();
                if nums.len() == 2 {
                    return format!("#[validate(range(min = {}, max = {}))]", nums[0], nums[1]);
                }
                if nums.len() == 1 {
                    return format!("#[validate(range(min = {}))]", nums[0]);
                }
            }
            if hint == "email" {
                return "#[validate(email)]".to_string();
            }
            if hint == "url" {
                return "#[validate(url)]".to_string();
            }
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

#[derive(Debug, Clone)]
struct ScopeConfig {
    field: String,
}

impl ScopeConfig {
    fn new(field: &str, label: &str) -> Result<Self, String> {
        validate_identifier(field, label)?;

        Ok(Self {
            field: field.to_string(),
        })
    }

    fn validate_against(&self, fields: &[Field]) -> Result<(), String> {
        let Some(field) = fields.iter().find(|field| field.name == self.field) else {
            return Err(format!(
                "Scope field '{}' must also be present in --fields.",
                self.field
            ));
        };

        if field.rust_type != "Uuid" {
            return Err(format!(
                "Scope field '{}' must be a uuid or ref field.",
                self.field
            ));
        }

        if !field.is_required {
            return Err(format!("Scope field '{}' must be required.", self.field));
        }

        Ok(())
    }
}

fn route_prefix(scopes: &[ScopeConfig]) -> String {
    scopes
        .iter()
        .map(|scope| {
            let resource = scope
                .field
                .strip_suffix("_id")
                .filter(|s| !s.is_empty())
                .unwrap_or(&scope.field);
            format!("/{}/{{{}}}", pluralize(resource), scope.field)
        })
        .collect()
}

fn scope_suffix(scopes: &[ScopeConfig]) -> String {
    scopes
        .iter()
        .map(|scope| scope.field.as_str())
        .collect::<Vec<_>>()
        .join("_and_")
}

fn scope_params(scopes: &[ScopeConfig]) -> String {
    scopes
        .iter()
        .map(|scope| format!("{}: Uuid", scope.field))
        .collect::<Vec<_>>()
        .join(",\n    ")
}

fn scope_args(scopes: &[ScopeConfig]) -> String {
    scopes
        .iter()
        .map(|scope| scope.field.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn scope_bind_lines(scopes: &[ScopeConfig], indent: &str) -> String {
    scopes
        .iter()
        .map(|scope| format!("{indent}.bind({})\n", scope.field))
        .collect()
}

fn scope_where_clause(scopes: &[ScopeConfig], starting_index: usize) -> String {
    scopes
        .iter()
        .enumerate()
        .map(|(idx, scope)| format!("{} = ${}", scope.field, starting_index + idx))
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn path_extractor(scopes: &[ScopeConfig], include_id: bool) -> String {
    let mut names: Vec<String> = scopes.iter().map(|scope| scope.field.clone()).collect();
    if include_id {
        names.push("id".to_string());
    }

    if names.len() == 1 {
        format!("Path({}): Path<Uuid>", names[0])
    } else {
        let types = vec!["Uuid"; names.len()].join(", ");
        format!("Path(({})): Path<({})>", names.join(", "), types)
    }
}

fn build_scopes(tenant: Option<&str>, scope_args: &[String]) -> Result<Vec<ScopeConfig>, String> {
    let mut fields = Vec::new();

    if let Some(tenant) = tenant {
        fields.push((tenant.to_string(), "tenant field"));
    }

    for scope in scope_args {
        fields.push((scope.clone(), "scope field"));
    }

    let mut scopes = Vec::new();
    for (field, label) in fields {
        if scopes
            .iter()
            .any(|scope: &ScopeConfig| scope.field == field)
        {
            continue;
        }
        scopes.push(ScopeConfig::new(&field, label)?);
    }

    Ok(scopes)
}

pub fn run(
    gen_type: &str,
    name: &str,
    field_args: &[String],
    tenant: Option<&str>,
    scope_args: &[String],
) {
    if gen_type != "model" && gen_type != "resource" {
        eprintln!(
            "❌ Invalid type: '{}'. Must be 'model' or 'resource'.",
            gen_type
        );
        std::process::exit(1);
    }

    let model_lower = name.to_lowercase();
    if let Err(e) = validate_identifier(&model_lower, "resource name") {
        eprintln!("❌ {}", e);
        std::process::exit(1);
    }

    let model_camel = to_camel_case(&model_lower);
    let table_name = pluralize(&model_lower);

    let fields: Vec<Field> = field_args
        .iter()
        .map(|a| Field::from_arg(a))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|e| {
            eprintln!("❌ {}", e);
            std::process::exit(1);
        });

    let scopes = build_scopes(tenant, scope_args).unwrap_or_else(|e| {
        eprintln!("❌ {}", e);
        std::process::exit(1);
    });

    if !scopes.is_empty() && gen_type != "resource" {
        eprintln!("❌ --tenant and --scope are only supported for generated resources.");
        std::process::exit(1);
    }

    for scope in &scopes {
        scope.validate_against(&fields).unwrap_or_else(|e| {
            eprintln!("❌ {}", e);
            std::process::exit(1);
        });
    }

    let prefix = project_prefix();
    let s = |path: &str| -> String { prefix.join(path).to_string_lossy().to_string() };
    let m_dir = migrations_dir();

    let domain_path = s(&format!("src/domain/{}.rs", model_lower));
    let domain_exists = Path::new(&domain_path).exists();

    println!(
        "🚀 Generating {} '{}' with {} field(s)...\n",
        gen_type,
        model_camel,
        fields.len()
    );

    if gen_type == "resource" && fields.is_empty() {
        println!("   ⚠️  No fields specified. Use --fields to add fields.");
        println!("   Example: --fields 'title:string:required:length(1,255)'\n");
    }

    if domain_exists {
        println!(
            "   ⚠️  Model '{}' already exists — skipping file and migration creation.\n",
            model_camel
        );
        println!("✅ Done!");
        return;
    }

    create_file(&domain_path, &domain_template(&model_camel, &fields));
    create_file(
        &s(&format!("src/repository/{}_repo.rs", model_lower)),
        &repo_template(
            &model_camel,
            &model_lower,
            &table_name,
            gen_type,
            &fields,
            &scopes,
        ),
    );
    append_mod(&s("src/domain/mod.rs"), &model_lower);
    append_mod(
        &s("src/repository/mod.rs"),
        &format!("{}_repo", model_lower),
    );

    if gen_type == "resource" {
        create_file(
            &s(&format!("src/http/dtos/{}_dto.rs", model_lower)),
            &dto_template(&model_camel, &model_lower, &fields, &scopes),
        );
        create_file(
            &s(&format!("src/http/handlers/{}_routes.rs", model_lower)),
            &handler_template(&model_camel, &model_lower, &scopes),
        );
        create_file(
            &s(&format!("src/services/{}_service.rs", model_lower)),
            &service_template(&model_camel, &model_lower, &scopes),
        );
        append_mod(&s("src/http/dtos/mod.rs"), &format!("{}_dto", model_lower));
        append_mod(
            &s("src/http/handlers/mod.rs"),
            &format!("{}_routes", model_lower),
        );
        append_mod(
            &s("src/services/mod.rs"),
            &format!("{}_service", model_lower),
        );
        inject_router(&prefix, &model_lower, &table_name, &scopes);
    }

    fs::create_dir_all(&m_dir).unwrap_or_else(|e| panic!("Failed to create migrations dir: {}", e));

    let trigger_path = m_dir.join("00000000000000_create_trigger_function.sql");
    create_file(&trigger_path.to_string_lossy(), TRIGGER_FUNCTION_SQL);

    let ver = next_migration_version(&m_dir);
    let mpath = m_dir.join(format!("{}_create_{}.sql", ver, table_name));
    create_file(
        &mpath.to_string_lossy(),
        &table_migration_sql(&table_name, &fields),
    );

    println!();
    println!("✅ Done!");
    println!();
    println!("📝 Next steps:");
    println!("   1. Run `cargo check`");
    println!(
        "   2. Run `rustwing run` (or `cargo run --bin api`) — migrations auto-run on startup"
    );
    if gen_type == "resource" {
        if !scopes.is_empty() {
            println!(
                "   3. Try: curl -X POST http://localhost:3000{}/{}",
                route_prefix(&scopes),
                table_name
            );
        } else {
            println!(
                "   3. Try: curl -X POST http://localhost:3000/{}",
                table_name
            );
        }
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

fn inject_router(prefix: &Path, lower: &str, plural: &str, scopes: &[ScopeConfig]) {
    let router_path = prefix.join("src/http/mod.rs");
    let content = fs::read_to_string(&router_path).expect("Failed to read http/mod.rs");

    if content.contains(&format!("handlers::{}_routes", lower)) {
        println!("⚠️  Skipped: router injection (routes already exist)");
        return;
    }

    let route_prefix = route_prefix(scopes);
    let new_routes = format!(
        "// {Model} routes\n        \
         .route(\"{route_prefix}/{plural}\", get(handlers::{lower}_routes::list_{lower}s).post(handlers::{lower}_routes::create_{lower}))\n        \
         .route(\"{route_prefix}/{plural}/cursor\", get(handlers::{lower}_routes::list_{lower}s_cursor))\n        \
         .route(\"{route_prefix}/{plural}/{{id}}\", get(handlers::{lower}_routes::get_{lower}).put(handlers::{lower}_routes::update_{lower}).delete(handlers::{lower}_routes::delete_{lower}))\n        \
         .with_state(state)",
        Model = to_camel_case(lower),
        route_prefix = route_prefix,
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

fn to_camel_case(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(uppercase_first)
        .collect()
}

fn next_migration_version(m_dir: &Path) -> String {
    let mut max_ver: u64 = 0;
    if m_dir.exists() {
        if let Ok(entries) = fs::read_dir(m_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(ver_str) = name.split('_').next() {
                    if let Ok(ver) = ver_str.parse::<u64>() {
                        max_ver = max_ver.max(ver);
                    }
                }
            }
        }
    }
    format!("{:014}", max_ver + 1)
}

fn table_migration_sql(table: &str, fields: &[Field]) -> String {
    let col_defs: String = fields
        .iter()
        .filter(|f| !f.is_auto())
        .map(|f| format!("{},\n", f.sql_column_def()))
        .collect();

    format!(
        r#"-- Create {table} table
CREATE TABLE IF NOT EXISTS {table} (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
{col_defs}    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

DROP TRIGGER IF EXISTS set_timestamp ON {table};
CREATE TRIGGER set_timestamp
BEFORE UPDATE ON {table}
FOR EACH ROW EXECUTE PROCEDURE trigger_set_timestamp();"#,
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

fn dto_template(model: &str, lower: &str, fields: &[Field], scopes: &[ScopeConfig]) -> String {
    let is_scope_field = |field: &Field| {
        scopes
            .iter()
            .any(|scope| scope.field.as_str() == field.name.as_str())
    };

    let create_fields: String = fields
        .iter()
        .filter(|f| !f.is_auto() && !is_scope_field(f))
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
        .filter(|f| !f.is_auto() && !is_scope_field(f))
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

fn scoped_repo_template(model: &str, table: &str, scopes: &[ScopeConfig]) -> String {
    if scopes.is_empty() {
        return String::new();
    }

    let suffix = scope_suffix(scopes);
    let params = scope_params(scopes);
    let args = scope_args(scopes);
    let where_clause = scope_where_clause(scopes, 1);
    let scope_binds = scope_bind_lines(scopes, "        ");
    let list_limit_idx = scopes.len() + 1;
    let list_offset_idx = scopes.len() + 2;
    let cursor_after_idx = scopes.len() + 1;
    let cursor_limit_idx = scopes.len() + 2;
    let id_idx = scopes.len() + 1;
    let update_scope_pushes: String = scopes
        .iter()
        .enumerate()
        .map(|(idx, scope)| {
            let prefix = if idx == 0 { " WHERE " } else { " AND " };
            format!(
                "    qb.push(\"{prefix}{field} = \").push_bind({field});\n",
                prefix = prefix,
                field = scope.field
            )
        })
        .collect();

    format!(
        r#"

pub async fn find_by_{suffix}(
    pool: &PgPool,
    {params},
    limit: i64,
    offset: i64,
) -> Result<Vec<{Model}>, CoreError> {{
    let query = "SELECT * FROM {table} WHERE {where_clause} LIMIT ${list_limit_idx} OFFSET ${list_offset_idx}";
    let records = sqlx::query_as(query)
{scope_binds}        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok(records)
}}

pub async fn find_by_{suffix}_cursor(
    pool: &PgPool,
    {params},
    after_id: Uuid,
    limit: i64,
) -> Result<Vec<{Model}>, CoreError> {{
    let query = "SELECT * FROM {table} WHERE {where_clause} AND id > ${cursor_after_idx} ORDER BY id LIMIT ${cursor_limit_idx}";
    let records = sqlx::query_as(query)
{scope_binds}        .bind(after_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(records)
}}

pub async fn find_by_{suffix}_and_id(
    pool: &PgPool,
    {params},
    id: Uuid,
) -> Result<{Model}, CoreError> {{
    let query = "SELECT * FROM {table} WHERE {where_clause} AND id = ${id_idx}";
    sqlx::query_as(query)
{scope_binds}        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(CoreError::NotFound)
}}

pub async fn update_by_{suffix}_and_id(
    pool: &PgPool,
    {params},
    id: Uuid,
    data: &{Model}Update,
) -> Result<{Model}, CoreError> {{
    let mut qb = QueryBuilder::new("UPDATE {table} SET ");
    if data.bind_updates(&mut qb) == UpdateResult::NoChanges {{
        return find_by_{suffix}_and_id(pool, {args}, id).await;
    }}

{update_scope_pushes}    qb.push(" AND id = ")
        .push_bind(id)
        .push(" RETURNING *");

    qb.build_query_as()
        .fetch_optional(pool)
        .await?
        .ok_or(CoreError::NotFound)
}}

pub async fn delete_by_{suffix}_and_id(
    pool: &PgPool,
    {params},
    id: Uuid,
) -> Result<(), CoreError> {{
    let query = "DELETE FROM {table} WHERE {where_clause} AND id = ${id_idx}";
    let result = sqlx::query(query)
{scope_binds}        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {{
        Err(CoreError::NotFound)
    }} else {{
        Ok(())
    }}
}}"#,
        Model = model,
        table = table,
        suffix = suffix,
        params = params,
        args = args,
        where_clause = where_clause,
        scope_binds = scope_binds,
        list_limit_idx = list_limit_idx,
        list_offset_idx = list_offset_idx,
        cursor_after_idx = cursor_after_idx,
        cursor_limit_idx = cursor_limit_idx,
        id_idx = id_idx,
        update_scope_pushes = update_scope_pushes,
    )
}

fn repo_template(
    model: &str,
    lower: &str,
    table: &str,
    gen_type: &str,
    fields: &[Field],
    scopes: &[ScopeConfig],
) -> String {
    let imports = if gen_type == "resource" {
        format!(
            "use crate::{{http::dtos::{lower}_dto::{{Create{Model}, Update{Model}}}, domain::{lower}::{Model}}};",
            lower = lower, Model = model
        )
    } else {
        format!(
            "use crate::domain::{lower}::{Model};",
            lower = lower,
            Model = model
        )
    };

    let resource_impls = if gen_type == "resource" {
        let data_fields: Vec<&Field> = fields.iter().filter(|f| !f.is_auto()).collect();
        let update_data_fields: Vec<&Field> = data_fields
            .iter()
            .copied()
            .filter(|f| {
                !scopes
                    .iter()
                    .any(|scope| scope.field.as_str() == f.name.as_str())
            })
            .collect();

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
            .filter(|f| {
                !scopes
                    .iter()
                    .any(|scope| scope.field.as_str() == f.name.as_str())
            })
            .map(|f| format!("    pub {}: Option<{}>,\n", f.name, f.rust_type))
            .collect();

        let from_insert_fields: String = data_fields
            .iter()
            .map(|f| {
                if scopes
                    .iter()
                    .any(|scope| scope.field.as_str() == f.name.as_str())
                {
                    format!("            {}: {},\n", f.name, f.name)
                } else {
                    format!("            {}: dto.{},\n", f.name, f.name)
                }
            })
            .collect();

        let from_insert_impl = if !scopes.is_empty() {
            format!(
                r#"impl Insert{Model} {{
    pub fn from_scopes(
    {scope_params},
        dto: Create{Model},
    ) -> Self {{
        Self {{
{from_insert_fields}        }}
    }}
}}"#,
                Model = model,
                scope_params = scope_params(scopes),
                from_insert_fields = from_insert_fields,
            )
        } else {
            format!(
                r#"impl From<Create{Model}> for Insert{Model} {{
    fn from(dto: Create{Model}) -> Self {{
        Self {{
{from_insert_fields}        }}
    }}
}}"#,
                Model = model,
                from_insert_fields = from_insert_fields,
            )
        };

        let from_update_fields: String = update_data_fields
            .iter()
            .map(|f| format!("            {}: dto.{},\n", f.name, f.name))
            .collect();

        let bind_values: String = data_fields
            .iter()
            .map(|f| format!("        separated.push_bind(&self.{});\n", f.name))
            .collect();

        let bind_updates: String = update_data_fields
            .iter()
            .map(|f| format!(
                "        if let Some(ref v) = self.{name} {{ separated.push(\"{name} = \").push_bind_unseparated(v); has_updates = true; }}\n",
                name = f.name
            ))
            .collect();

        let scoped_repo = scoped_repo_template(model, table, scopes);

        format!(
            r#"

pub struct Insert{Model} {{
{insert_fields}}}

{from_insert_impl}

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
}}{scoped_repo}"#,
            Model = model,
            insert_fields = insert_fields,
            update_fields = update_fields,
            columns = columns,
            bind_values = bind_values,
            bind_updates = bind_updates,
            from_insert_impl = from_insert_impl,
            from_update_fields = from_update_fields,
            scoped_repo = scoped_repo,
        )
    } else {
        String::new()
    };

    let sqlx_import = if gen_type == "resource" && !scopes.is_empty() {
        "use sqlx::{PgPool, Postgres, QueryBuilder};\nuse uuid::Uuid;\n"
    } else if gen_type == "resource" {
        "use sqlx::{Postgres, QueryBuilder};\n"
    } else {
        ""
    };

    format!(
        r#"{sqlx_import}use rustwing::prelude::*;
{imports}

impl ModelName for {Model} {{
    fn table_name() -> &'static str {{ "{table}" }}
}}{resource_impls}
"#,
        Model = model,
        table = table,
        imports = imports,
        resource_impls = resource_impls,
        sqlx_import = sqlx_import,
    )
}

fn handler_template(model: &str, lower: &str, scopes: &[ScopeConfig]) -> String {
    if !scopes.is_empty() {
        let scoped_path = path_extractor(scopes, false);
        let scoped_id_path = path_extractor(scopes, true);
        let args = scope_args(scopes);
        return format!(
            r#"use axum::{{extract::{{Path, Query, State}}, http::StatusCode, Json}};
use uuid::Uuid;

use crate::{{
    error::AppError,
    http::dtos::{lower}_dto::{{Create{Model}, Update{Model}, {Model}Response}},
    http::extractors::AuthUser,
    http::handlers::user_routes::{{Pagination, CursorPagination}},
    services::{lower}_service,
    state::AppState,
}};

pub async fn list_{lower}s(
    _auth: AuthUser,
    State(state): State<AppState>,
    {scoped_path},
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<{Model}Response>>, AppError> {{
    let items = {lower}_service::list_{lower}s(&state.db, {args}, p.limit, p.offset).await?;
    Ok(Json(items.into_iter().map({Model}Response::from).collect()))
}}

pub async fn list_{lower}s_cursor(
    _auth: AuthUser,
    State(state): State<AppState>,
    {scoped_path},
    Query(p): Query<CursorPagination>,
) -> Result<Json<Vec<{Model}Response>>, AppError> {{
    let items = {lower}_service::list_{lower}s_cursor(&state.db, {args}, p.after, p.limit).await?;
    Ok(Json(items.into_iter().map({Model}Response::from).collect()))
}}

pub async fn get_{lower}(
    _auth: AuthUser,
    State(state): State<AppState>,
    {scoped_id_path},
) -> Result<Json<{Model}Response>, AppError> {{
    let item = {lower}_service::get_{lower}(&state.db, {args}, id).await?;
    Ok(Json({Model}Response::from(item)))
}}

pub async fn create_{lower}(
    _auth: AuthUser,
    State(state): State<AppState>,
    {scoped_path},
    Json(payload): Json<Create{Model}>,
) -> Result<(StatusCode, Json<{Model}Response>), AppError> {{
    let item = {lower}_service::create_{lower}(&state.db, {args}, payload).await?;
    Ok((StatusCode::CREATED, Json({Model}Response::from(item))))
}}

pub async fn update_{lower}(
    _auth: AuthUser,
    State(state): State<AppState>,
    {scoped_id_path},
    Json(payload): Json<Update{Model}>,
) -> Result<Json<{Model}Response>, AppError> {{
    let item = {lower}_service::update_{lower}(&state.db, {args}, id, payload).await?;
    Ok(Json({Model}Response::from(item)))
}}

pub async fn delete_{lower}(
    _auth: AuthUser,
    State(state): State<AppState>,
    {scoped_id_path},
) -> Result<StatusCode, AppError> {{
    {lower}_service::delete_{lower}(&state.db, {args}, id).await?;
    Ok(StatusCode::NO_CONTENT)
}}
"#,
            Model = model,
            lower = lower,
            scoped_path = scoped_path,
            scoped_id_path = scoped_id_path,
            args = args,
        );
    }

    format!(
        r#"use axum::{{extract::{{Path, Query, State}}, http::StatusCode, Json}};
use uuid::Uuid;

use crate::{{
    error::AppError,
    http::dtos::{lower}_dto::{{Create{Model}, Update{Model}, {Model}Response}},
    http::extractors::AuthUser,
    http::handlers::user_routes::{{Pagination, CursorPagination}},
    services::{lower}_service,
    state::AppState,
}};

pub async fn list_{lower}s(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(p): Query<Pagination>,
) -> Result<Json<Vec<{Model}Response>>, AppError> {{
    let items = {lower}_service::list_{lower}s(&state.db, p.limit, p.offset).await?;
    Ok(Json(items.into_iter().map({Model}Response::from).collect()))
}}

pub async fn list_{lower}s_cursor(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(p): Query<CursorPagination>,
) -> Result<Json<Vec<{Model}Response>>, AppError> {{
    let items = {lower}_service::list_{lower}s_cursor(&state.db, p.after, p.limit).await?;
    Ok(Json(items.into_iter().map({Model}Response::from).collect()))
}}

pub async fn get_{lower}(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<{Model}Response>, AppError> {{
    let item = {lower}_service::get_{lower}(&state.db, id).await?;
    Ok(Json({Model}Response::from(item)))
}}

pub async fn create_{lower}(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<Create{Model}>,
) -> Result<(StatusCode, Json<{Model}Response>), AppError> {{
    let item = {lower}_service::create_{lower}(&state.db, payload).await?;
    Ok((StatusCode::CREATED, Json({Model}Response::from(item))))
}}

pub async fn update_{lower}(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<Update{Model}>,
) -> Result<Json<{Model}Response>, AppError> {{
    let item = {lower}_service::update_{lower}(&state.db, id, payload).await?;
    Ok(Json({Model}Response::from(item)))
}}

pub async fn delete_{lower}(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {{
    {lower}_service::delete_{lower}(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}}
"#,
        Model = model,
        lower = lower,
    )
}

fn service_template(model: &str, lower: &str, scopes: &[ScopeConfig]) -> String {
    if !scopes.is_empty() {
        let suffix = scope_suffix(scopes);
        let params = scope_params(scopes);
        let args = scope_args(scopes);
        return format!(
            r#"use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::{{
    domain::{lower}::{Model},
    error::AppError,
    http::dtos::{lower}_dto::{{Create{Model}, Update{Model}}},
    repository::{lower}_repo::{{self, Insert{Model}, {Model}Update}},
}};
use rustwing::prelude::*;

const DEFAULT_LIMIT: i64 = 10;
const MAX_LIMIT: i64 = 100;

pub async fn list_{lower}s(
    db: &PgPool,
    {params},
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<{Model}>, AppError> {{
    Ok({lower}_repo::find_by_{suffix}(
        db,
        {args},
        normalize_limit(limit),
        normalize_offset(offset),
    )
    .await?)
}}

pub async fn list_{lower}s_cursor(
    db: &PgPool,
    {params},
    after: Option<Uuid>,
    limit: Option<i64>,
) -> Result<Vec<{Model}>, AppError> {{
    Ok({lower}_repo::find_by_{suffix}_cursor(
        db,
        {args},
        after.unwrap_or_else(Uuid::nil),
        normalize_limit(limit),
    )
    .await?)
}}

pub async fn get_{lower}(
    db: &PgPool,
    {params},
    id: Uuid,
) -> Result<{Model}, AppError> {{
    Ok({lower}_repo::find_by_{suffix}_and_id(db, {args}, id).await?)
}}

pub async fn create_{lower}(
    db: &PgPool,
    {params},
    payload: Create{Model},
) -> Result<{Model}, AppError> {{
    payload.validate()?;
    let insert = Insert{Model}::from_scopes({args}, payload);
    Ok(generic_crud::insert::<{Model}, Insert{Model}>(db, &insert).await?)
}}

pub async fn update_{lower}(
    db: &PgPool,
    {params},
    id: Uuid,
    payload: Update{Model},
) -> Result<{Model}, AppError> {{
    let update = {Model}Update::from(payload);
    Ok({lower}_repo::update_by_{suffix}_and_id(db, {args}, id, &update).await?)
}}

pub async fn delete_{lower}(
    db: &PgPool,
    {params},
    id: Uuid,
) -> Result<(), AppError> {{
    Ok({lower}_repo::delete_by_{suffix}_and_id(db, {args}, id).await?)
}}

fn normalize_limit(limit: Option<i64>) -> i64 {{
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}}

fn normalize_offset(offset: Option<i64>) -> i64 {{
    offset.unwrap_or(0).max(0)
}}
"#,
            Model = model,
            lower = lower,
            params = params,
            args = args,
            suffix = suffix,
        );
    }

    format!(
        r#"use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::{{
    domain::{lower}::{Model},
    error::AppError,
    http::dtos::{lower}_dto::{{Create{Model}, Update{Model}}},
    repository::{lower}_repo::{{Insert{Model}, {Model}Update}},
}};
use rustwing::prelude::*;

const DEFAULT_LIMIT: i64 = 10;
const MAX_LIMIT: i64 = 100;

pub async fn list_{lower}s(
    db: &PgPool,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<{Model}>, AppError> {{
    Ok(generic_crud::find_all::<{Model}>(db, normalize_limit(limit), normalize_offset(offset)).await?)
}}

pub async fn list_{lower}s_cursor(
    db: &PgPool,
    after: Option<Uuid>,
    limit: Option<i64>,
) -> Result<Vec<{Model}>, AppError> {{
    Ok(generic_crud::find_after::<{Model}>(
        db,
        after.unwrap_or_else(Uuid::nil),
        normalize_limit(limit),
    )
    .await?)
}}

pub async fn get_{lower}(db: &PgPool, id: Uuid) -> Result<{Model}, AppError> {{
    Ok(generic_crud::find_by_id::<{Model}>(db, id).await?)
}}

pub async fn create_{lower}(db: &PgPool, payload: Create{Model}) -> Result<{Model}, AppError> {{
    payload.validate()?;
    let insert = Insert{Model}::from(payload);
    Ok(generic_crud::insert::<{Model}, Insert{Model}>(db, &insert).await?)
}}

pub async fn update_{lower}(
    db: &PgPool,
    id: Uuid,
    payload: Update{Model},
) -> Result<{Model}, AppError> {{
    let update = {Model}Update::from(payload);
    Ok(generic_crud::update::<{Model}, {Model}Update>(db, id, &update).await?)
}}

pub async fn delete_{lower}(db: &PgPool, id: Uuid) -> Result<(), AppError> {{
    Ok(generic_crud::delete::<{Model}>(db, id).await?)
}}

fn normalize_limit(limit: Option<i64>) -> i64 {{
    limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
}}

fn normalize_offset(offset: Option<i64>) -> i64 {{
    offset.unwrap_or(0).max(0)
}}
"#,
        Model = model,
        lower = lower,
    )
}
