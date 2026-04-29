use include_dir::{include_dir, Dir, DirEntry};
use std::path::Path;

static TEMPLATE: Dir = include_dir!("$CARGO_MANIFEST_DIR/template");

pub fn run(project_name: &str) {
    let dest = Path::new(project_name);
    if dest.exists() {
        eprintln!("❌ Directory '{}' already exists.", project_name);
        std::process::exit(1);
    }

    println!("🚀 Creating Rustwing project '{}'...\n", project_name);

    copy_dir(&TEMPLATE, dest, project_name);

    println!("✅ Done!");
    println!();
    println!("📝 Next steps:");
    println!("   cd {}", project_name);
    println!("   cargo build");
    println!("   # Set up your database and run:");
    println!("   cargo run --bin api");
}

fn copy_dir(dir: &Dir, dest: &Path, project_name: &str) {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(subdir) => {
                let sub_path = dest.join(subdir.path());
                std::fs::create_dir_all(&sub_path).unwrap_or_else(|e| {
                    eprintln!("❌ Failed to create directory {}: {}", sub_path.display(), e);
                    std::process::exit(1);
                });
                copy_dir(subdir, dest, project_name);
            }
            DirEntry::File(file) => {
                let file_path = dest.join(file.path());
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                        eprintln!("❌ Failed to create directory {}: {}", parent.display(), e);
                        std::process::exit(1);
                    });
                }
                match file.contents_utf8() {
                    Some(content) => {
                        let processed = content.replace("{{project_name}}", project_name);
                        std::fs::write(&file_path, processed).unwrap_or_else(|e| {
                            eprintln!("❌ Failed to write {}: {}", file_path.display(), e);
                            std::process::exit(1);
                        });
                    }
                    None => {
                        std::fs::write(&file_path, file.contents()).unwrap_or_else(|e| {
                            eprintln!("❌ Failed to write {}: {}", file_path.display(), e);
                            std::process::exit(1);
                        });
                    }
                }
                println!("   📄 Created: {}", file.path().display());
            }
        }
    }
}
