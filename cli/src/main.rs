mod generate;
mod new;
mod template_data;

use clap::{Parser, Subcommand};
use std::path::Path;
use std::process::Command;

const VERSION_INFO: &str = concat!(
    "CLI ",
    env!("CARGO_PKG_VERSION"),
    "\nrustwing framework ",
    "0.1.2"
);

#[derive(Parser)]
#[command(name = "rustwing", about = "Rustwing CLI", version = VERSION_INFO)]
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
        /// Path to local rustwing checkout for development (uses path dependency instead of crates.io)
        #[arg(long)]
        local: Option<String>,
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
        /// Tenant scope column for SaaS resources, e.g. org_id
        #[arg(long)]
        tenant: Option<String>,
        /// Parent/scope column for nested SQLx helpers and routes, e.g. ticket_id
        #[arg(long = "scope", num_args = 1)]
        scopes: Vec<String>,
        /// Fields in format: name:type:required|optional[:validator]
        #[arg(long = "fields", num_args = 1)]
        fields: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::New { name, local } => new::run(&name, local.as_deref()),
        Commands::Run => run(),
        Commands::Generate {
            r#type,
            name,
            tenant,
            scopes,
            fields,
        } => generate::run(&r#type, &name, &fields, tenant.as_deref(), &scopes),
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
