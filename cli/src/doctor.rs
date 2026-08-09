use std::collections::HashSet;
use std::fs;
use std::path::Path;

const FRAMEWORK_VERSION: &str = "0.1.10";

#[derive(Debug, Default)]
struct Report {
    errors: Vec<String>,
    warnings: Vec<String>,
    ok: Vec<String>,
}

impl Report {
    fn check(&mut self, condition: bool, message: impl Into<String>) {
        if condition {
            self.ok.push(message.into());
        } else {
            self.errors.push(message.into());
        }
    }

    fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }
}

pub fn run() {
    let report = inspect(Path::new("."));

    println!("Rustwing doctor\n");
    for message in &report.ok {
        println!("✅ {message}");
    }
    for message in &report.warnings {
        println!("⚠️  {message}");
    }
    for message in &report.errors {
        println!("❌ {message}");
    }

    if !report.errors.is_empty() {
        std::process::exit(1);
    }

    println!("\nDoctor found no blocking issues.");
}

fn inspect(root: &Path) -> Report {
    let mut report = Report::default();
    report.check(
        root.join("Cargo.toml").is_file(),
        "workspace Cargo.toml found",
    );
    report.check(
        root.join("api/Cargo.toml").is_file(),
        "generated API crate found",
    );

    let metadata_path = root.join(".rustwing-version");
    if !metadata_path.is_file() {
        report
            .warn(".rustwing-version is missing; regenerate the project or add scaffold metadata");
    } else if let Ok(content) = fs::read_to_string(&metadata_path) {
        let metadata = parse_metadata(&content);
        check_version(
            &mut report,
            &metadata,
            "framework_version",
            FRAMEWORK_VERSION,
        );
        check_version(
            &mut report,
            &metadata,
            "cli_version",
            env!("CARGO_PKG_VERSION"),
        );
        check_version(
            &mut report,
            &metadata,
            "template_version",
            env!("CARGO_PKG_VERSION"),
        );
    } else {
        report
            .errors
            .push(".rustwing-version cannot be read".to_string());
    }

    if let Ok(cargo) = fs::read_to_string(root.join("api/Cargo.toml")) {
        report.check(
            cargo
                .lines()
                .any(|line| line.trim_start().starts_with("rustwing")),
            "API declares a rustwing framework dependency",
        );
        check_dependency(&mut report, &cargo, "sqlx", "0.9");
        check_dependency(&mut report, &cargo, "jsonwebtoken", "11");
        check_dependency(&mut report, &cargo, "validator", "0.19");
    } else {
        report
            .errors
            .push("API Cargo.toml cannot be read".to_string());
    }

    if let Ok(worker_cargo) = fs::read_to_string(root.join("worker/Cargo.toml")) {
        check_dependency(&mut report, &worker_cargo, "sqlx", "0.9");
    } else {
        report.warn("worker/Cargo.toml is missing; this may be intentional for a custom project");
    }

    check_marker(
        &mut report,
        root,
        "api/src/http/mod.rs",
        "// rustwing:routes",
    );
    for marker in [
        "// rustwing:openapi-paths",
        "// rustwing:openapi-schemas",
        "// rustwing:openapi-tags",
    ] {
        check_marker(&mut report, root, "api/src/openapi.rs", marker);
    }

    let migrations = root.join("api/migrations");
    report.check(migrations.is_dir(), "API migrations directory found");
    if migrations.is_dir() {
        let mut versions = HashSet::new();
        let mut migration_count = 0;
        if let Ok(entries) = fs::read_dir(&migrations) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.ends_with(".sql") {
                    continue;
                }
                migration_count += 1;
                let version = name.split('_').next().unwrap_or_default();
                if !version.is_empty() && !versions.insert(version.to_string()) {
                    report
                        .errors
                        .push(format!("duplicate migration version: {version}"));
                }
            }
        }
        report.check(migration_count > 0, "at least one SQL migration found");
        report.check(
            migrations
                .join("00000000000000_create_trigger_function.sql")
                .is_file(),
            "shared timestamp trigger migration found",
        );
    }

    if let Ok(env_example) = fs::read_to_string(root.join(".env.example")) {
        report.check(
            env_example.contains("DATABASE_URL="),
            "DATABASE_URL documented in .env.example",
        );
        report.check(
            env_example.contains("JWT_SECRET="),
            "JWT_SECRET documented in .env.example",
        );
    } else {
        report.warn(".env.example is missing");
    }

    report
}

fn check_dependency(report: &mut Report, cargo: &str, name: &str, expected: &str) {
    let Some(line) = cargo
        .lines()
        .find(|line| line.trim_start().starts_with(&format!("{name} =")))
    else {
        report.warn(format!("{name} dependency is not declared directly"));
        return;
    };

    let actual = line
        .split("version = \"")
        .nth(1)
        .and_then(|value| value.split('"').next())
        .or_else(|| {
            line.split("= \"")
                .nth(1)
                .and_then(|value| value.split('"').next())
        });

    match actual {
        Some(version) if version == expected => {
            report
                .ok
                .push(format!("{name} dependency baseline is {version}"));
        }
        Some(version) => report.warnings.push(format!(
            "{name} dependency is {version}; current baseline is {expected}"
        )),
        None => report.warnings.push(format!(
            "could not determine {name} dependency version; inspect {line}"
        )),
    }
}

fn check_marker(report: &mut Report, root: &Path, relative: &str, marker: &str) {
    match fs::read_to_string(root.join(relative)) {
        Ok(content) if content.contains(marker) => {
            report.ok.push(format!("generator marker found: {relative}"));
        }
        Ok(_) => report.errors.push(format!(
            "generator marker `{marker}` is missing from {relative}; future generation may be unsafe"
        )),
        Err(_) => report
            .errors
            .push(format!("generated file is missing: {relative}")),
    }
}

fn parse_metadata(content: &str) -> std::collections::HashMap<&str, String> {
    content
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let value = value.trim().trim_matches('"').to_string();
            Some((key.trim(), value))
        })
        .collect()
}

fn check_version(
    report: &mut Report,
    metadata: &std::collections::HashMap<&str, String>,
    key: &str,
    expected: &str,
) {
    match metadata.get(key) {
        Some(actual) if actual == expected => report.ok.push(format!("{key} is {actual}")),
        Some(actual) => report.warn(format!("{key} is {actual}; current CLI expects {expected}")),
        None => report
            .errors
            .push(format!("{key} is missing from .rustwing-version")),
    }
}
