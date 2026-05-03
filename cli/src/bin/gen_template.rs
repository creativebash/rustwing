use std::fs;
use std::path::Path;

fn main() {
    let template_dir = Path::new("template");
    let mut entries: Vec<(String, String)> = Vec::new();

    collect_files(template_dir, template_dir, &mut entries);
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::new();
    out.push_str("// Auto-generated. Do not edit manually.\n");
    out.push_str("// Regenerate: cd cli && cargo run --bin gen-template\n\n");
    out.push_str("pub static FILES: &[(&str, &str)] = &[\n");

    for (rel, content) in &entries {
        let escaped_rel = escape_rust_string(rel);
        let escaped_content = escape_rust_string(content);
        out.push_str(&format!("    ({}, {}),\n", escaped_rel, escaped_content));
    }

    out.push_str("];\n");

    fs::write("src/template_data.rs", &out).expect("Failed to write src/template_data.rs");
    println!("Generated {} template entries in src/template_data.rs", entries.len());
}

fn collect_files(base: &Path, dir: &Path, entries: &mut Vec<(String, String)>) {
    let Ok(read_dir) = fs::read_dir(dir) else { return };
    let mut dirs: Vec<_> = Vec::new();
    let mut files: Vec<_> = Vec::new();

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        } else if path.is_file() {
            files.push(path);
        }
    }

    dirs.sort();
    files.sort();

    for path in &files {
        let rel = path.strip_prefix(base).unwrap().to_string_lossy().to_string();
        let content = fs::read_to_string(path).unwrap();
        entries.push((rel, content));
    }

    for path in &dirs {
        collect_files(base, path, entries);
    }
}

fn escape_rust_string(s: &str) -> String {
    format!("{:?}", s)
}
