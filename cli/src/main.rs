mod generate;
mod new;
mod template_data;

use clap::{Parser, Subcommand};
use std::path::Path;
use std::process::Command;

#[derive(Parser)]
#[command(
    name = "rustwing",
    about = "Rustwing CLI",
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Rustwing project
    New {
        /// Project name
        name: String,
    },
    /// Run the API server (cargo run --bin api)
    Run,
    /// Generate a resource, model, etc.
    #[command(alias = "g")]
    Generate {
        /// Type: resource or model
        r#type: String,
        /// Name of the resource (e.g. post, product)
        name: String,
        /// Fields in format: name:type:required|optional[:validator]
        #[arg(long = "fields", num_args = 1)]
        fields: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::New { name } => new::run(&name),
        Commands::Run => run(),
        Commands::Generate { r#type, name, fields } => generate::run(&r#type, &name, &fields),
    }
}

fn run() {
    if !Path::new("Cargo.toml").exists() {
        eprintln!("❌ No Cargo.toml found. Run this from a Rustwing project root.");
        std::process::exit(1);
    }

    let status = Command::new("cargo")
        .args(["run", "--bin", "api"])
        .status()
        .expect("Failed to run cargo — is it installed?");

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
