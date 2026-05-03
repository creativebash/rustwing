use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn run(project_name: &str, local_framework: Option<&str>) {
    let dest = Path::new(project_name);
    if dest.exists() {
        eprintln!("❌ Directory '{}' already exists.", project_name);
        std::process::exit(1);
    }

    println!("🚀 Creating Rustwing project '{}'...\n", project_name);

    for &(path, content) in crate::template_data::FILES {
        let file_path = dest.join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("❌ Failed to create directory {}: {}", parent.display(), e);
                std::process::exit(1);
            });
        }
        let processed = content.replace("{{project_name}}", project_name);
        fs::write(&file_path, processed).unwrap_or_else(|e| {
            eprintln!("❌ Failed to write {}: {}", file_path.display(), e);
            std::process::exit(1);
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
            .unwrap_or_else(|e| panic!("Failed to open {}: {}", cargo_path.display(), e));
        file.write_all(patch.as_bytes()).unwrap();
        println!("   🔧 Patched: {} → local rustwing at {}", cargo_path.display(), framework_path.display());
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
        let cwd = std::env::current_dir().expect("Failed to get current directory");
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
