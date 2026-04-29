use std::fs;
use std::path::Path;

pub fn run(project_name: &str) {
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

    println!("✅ Done!");
    println!();
    println!("📝 Next steps:");
    println!("   cd {}", project_name);
    println!("   cargo build");
    println!("   # Set up your database and run:");
    println!("   cargo run --bin api");
}
