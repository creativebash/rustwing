use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run(apply: bool) {
    let root = Path::new(".");
    if !root.join("Cargo.toml").is_file() {
        eprintln!("❌ No Cargo.toml found. Run this from a Rustwing project root.");
        std::process::exit(1);
    }

    println!("Rustwing upgrade plan\n");
    println!("• Update the rustwing dependency lockfile");
    println!("• Compile the project with cargo check");
    println!("• Refresh .rustwing-version only after verification");
    println!("• Leave application code, migrations, and custom files untouched");

    if !apply {
        println!("\nDry run only. Re-run with `rustwing upgrade --apply` to execute this plan.");
        return;
    }

    let update = Command::new("cargo")
        .args(["update", "-p", "rustwing"])
        .status()
        .unwrap_or_else(|error| fail(format!("failed to run cargo update: {error}")));
    if !update.success() {
        fail("cargo update -p rustwing failed; no metadata was changed");
    }

    let check = Command::new("cargo")
        .arg("check")
        .status()
        .unwrap_or_else(|error| fail(format!("failed to run cargo check: {error}")));
    if !check.success() {
        fail("cargo check failed; review the compiler output before retrying");
    }

    let metadata_path = root.join(".rustwing-version");
    let metadata = format!(
        "format = 1\ncli_version = \"{}\"\nframework_version = \"{}\"\ntemplate_version = \"{}\"\n",
        env!("CARGO_PKG_VERSION"),
        crate::FRAMEWORK_VERSION,
        env!("CARGO_PKG_VERSION")
    );
    fs::write(&metadata_path, metadata).unwrap_or_else(|error| {
        fail(format!(
            "failed to write {}: {error}",
            metadata_path.display()
        ))
    });

    println!("\n✅ Upgrade verification passed; .rustwing-version was refreshed.");
}

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("❌ {message}");
    std::process::exit(1);
}
