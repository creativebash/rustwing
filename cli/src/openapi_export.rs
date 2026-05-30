use std::fs;
use std::path::Path;
use std::process::Command;

const DEFAULT_OUTPUT: &str = "openapi/openapi.json";

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("❌ {}", message);
    std::process::exit(1);
}

pub fn generate_json() -> Result<String, String> {
    if !Path::new("Cargo.toml").exists() {
        return Err("No Cargo.toml found. Run this from a Rustwing project root.".to_string());
    }

    let output = Command::new("cargo")
        .args(["run", "--quiet", "--bin", "api", "--", "--openapi-json"])
        .output()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Failed to generate OpenAPI JSON via `cargo run --bin api -- --openapi-json`.\n{}",
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("OpenAPI output was not valid UTF-8: {}", e))?;
    let value: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("OpenAPI output was not valid JSON: {}", e))?;
    serde_json::to_string_pretty(&value)
        .map(|json| format!("{}\n", json))
        .map_err(|e| format!("Failed to format OpenAPI JSON: {}", e))
}

pub fn run(output: Option<&str>, check: bool, stdout: bool) {
    if check && stdout {
        fail("Use either --check or --stdout, not both.");
    }

    let json = generate_json().unwrap_or_else(|e| fail(e));

    if stdout {
        print!("{}", json);
        return;
    }

    let output_path = output.unwrap_or(DEFAULT_OUTPUT);
    let path = Path::new(output_path);

    if check {
        let existing = fs::read_to_string(path).unwrap_or_default();
        if existing != json {
            fail(format!(
                "{} is out of date. Run `rustwing g openapi --output {}` to refresh it.",
                path.display(),
                path.display()
            ));
        }
        println!("OpenAPI spec is up to date: {}", path.display());
        return;
    }

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .unwrap_or_else(|e| fail(format!("Failed to create {}: {}", parent.display(), e)));
    }

    fs::write(path, json)
        .unwrap_or_else(|e| fail(format!("Failed to write {}: {}", path.display(), e)));
    println!("📘 Wrote OpenAPI spec to {}", path.display());
}
