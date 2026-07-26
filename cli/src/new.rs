use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("❌ {}", message);
    std::process::exit(1);
}

pub fn run(project_name: &str, local_framework: Option<&str>) {
    let dest = Path::new(project_name);
    if dest.exists() {
        fail(format!("Directory '{}' already exists.", project_name));
    }

    println!("🚀 Creating Rustwing project '{}'...\n", project_name);

    for &(path, content) in crate::template_data::FILES {
        let file_path = dest.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                fail(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ));
            });
        }
        let processed = content
            .replace("{{project_name}}", project_name)
            .replace("{{cli_version}}", env!("CARGO_PKG_VERSION"))
            .replace("{{framework_version}}", crate::FRAMEWORK_VERSION);
        fs::write(&file_path, processed).unwrap_or_else(|e| {
            fail(format!("Failed to write {}: {}", file_path.display(), e));
        });
        println!("   📄 Created: {}", path);
    }

    if let Some(local) = local_framework {
        let framework_path = resolve_framework_path(local);
        let patch = format!(
            "\n[patch.crates-io]\nrustwing = {{ path = {} }}\n",
            escape_path(&framework_path)
        );
        let cargo_path = dest.join("Cargo.toml");
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&cargo_path)
            .unwrap_or_else(|e| fail(format!("Failed to open {}: {}", cargo_path.display(), e)));
        file.write_all(patch.as_bytes())
            .unwrap_or_else(|e| fail(format!("Failed to write {}: {}", cargo_path.display(), e)));
        println!(
            "   🔧 Patched: {} → local rustwing at {}",
            cargo_path.display(),
            framework_path.display()
        );
    }

    println!();
    println!("✅ Done!");
    println!();
    println!("📝 Next steps:");
    println!("   cd {}", project_name);
    println!("   cargo build");
    println!("   # Set up your database and run:");
    println!("   cargo run --bin api   # or: rustwing run");
}

fn resolve_framework_path(path: &str) -> PathBuf {
    let p = Path::new(path);
    let base = if p.is_absolute() {
        p.to_path_buf()
    } else {
        let cwd = std::env::current_dir()
            .unwrap_or_else(|e| fail(format!("Failed to get current directory: {}", e)));
        cwd.join(p)
    };

    // If user passed repo root (contains rustwing/ subdir), resolve to the crate
    let candidate = base.join("rustwing");
    if candidate.join("Cargo.toml").exists() {
        candidate
    } else {
        base
    }
}

fn escape_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

#[cfg(test)]
mod tests {
    fn template(path: &str) -> &'static str {
        crate::template_data::FILES
            .iter()
            .find_map(|(candidate, content)| (*candidate == path).then_some(*content))
            .unwrap_or_else(|| panic!("missing embedded template: {path}"))
    }

    #[test]
    fn starter_accounts_are_self_only() {
        let routes = template("api/src/http/handlers/user_routes.rs");

        assert!(routes.contains("path = \"/users/me\""));
        assert!(!routes.contains("path = \"/users/{id}\""));
        assert!(!routes.contains("list_users"));
    }

    #[test]
    fn public_user_model_cannot_serialize_password_hashes() {
        let user = template("api/src/domain/user.rs");
        let record = template("api/src/repository/user_repo.rs");

        assert!(!user.contains("password_hash"));
        assert!(record.contains("struct UserRecord"));
        assert!(record.contains("password_hash"));
    }

    #[test]
    fn startup_configuration_and_migrations_fail_closed() {
        let main = template("api/src/main.rs");

        assert!(main.contains("JWT_SECRET must be set"));
        assert!(!main.contains("super_secret_dev_key_change_me"));
        assert!(!main.contains("DELETE FROM _sqlx_migrations"));
    }

    #[test]
    fn scaffold_embeds_upgrade_metadata() {
        let metadata = template(".rustwing-version");

        assert!(metadata.contains("cli_version = \"{{cli_version}}\""));
        assert!(metadata.contains("framework_version = \"{{framework_version}}\""));
        assert!(metadata.contains("template_version = \"{{cli_version}}\""));
    }

    #[test]
    fn starter_tenant_authorization_is_wired() {
        let migration = template("api/migrations/00000000000002_create_organizations.sql");
        let authorization = template("api/src/services/authorization.rs");

        assert!(migration.contains("organization_members"));
        assert!(migration.contains("status TEXT NOT NULL DEFAULT 'active'"));
        assert!(authorization.contains("require_membership"));
        assert!(authorization.contains("CoreError::Forbidden"));
    }
}
