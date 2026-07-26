mod doctor;
mod generate;
mod new;
mod openapi_export;
mod template_data;
mod ts_client;
mod upgrade;

use clap::{Parser, Subcommand};
use std::path::Path;
use std::process::Command;

// When releasing: bump CLI_VERSION in cli/Cargo.toml, bump FRAMEWORK_VERSION below
const FRAMEWORK_VERSION: &str = "0.1.8";
const VERSION_INFO: &str = concat!(
    "CLI ",
    env!("CARGO_PKG_VERSION"),
    "\nrustwing framework ",
    "0.1.8", // FRAMEWORK_VERSION — bump when rustwing/ releases
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
    /// Inspect a generated project for upgrade and configuration drift
    Doctor,
    /// Preview or apply a verified framework dependency upgrade
    Upgrade {
        /// Apply the upgrade after updating the lockfile and passing cargo check
        #[arg(long)]
        apply: bool,
    },
    /// Generate a resource, model, etc.
    #[command(alias = "g")]
    Generate {
        /// Generator: resource, model, openapi, or client
        r#type: String,
        /// Name of the resource/model, or client target for `g client typescript`
        name: Option<String>,
        /// Tenant scope column for SaaS resources, e.g. org_id
        #[arg(long)]
        tenant: Option<String>,
        /// Parent/scope column for nested SQLx helpers and routes, e.g. ticket_id
        #[arg(long = "scope", num_args = 1)]
        scopes: Vec<String>,
        /// Fields in format: name:type:required|optional[:validator]
        #[arg(long = "fields", num_args = 1)]
        fields: Vec<String>,
        /// Output path for generated OpenAPI or client artifacts
        #[arg(long)]
        output: Option<String>,
        /// Check whether generated OpenAPI output is up to date
        #[arg(long)]
        check: bool,
        /// Print generated OpenAPI JSON to stdout
        #[arg(long)]
        stdout: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::New { name, local } => new::run(&name, local.as_deref()),
        Commands::Run => run(),
        Commands::Doctor => doctor::run(),
        Commands::Upgrade { apply } => upgrade::run(apply),
        Commands::Generate {
            r#type,
            name,
            tenant,
            scopes,
            fields,
            output,
            check,
            stdout,
        } => generate_command(GenerateCommandArgs {
            gen_type: &r#type,
            name: name.as_deref(),
            fields: &fields,
            tenant: tenant.as_deref(),
            scopes: &scopes,
            output: output.as_deref(),
            check,
            stdout,
        }),
    }
}

struct GenerateCommandArgs<'a> {
    gen_type: &'a str,
    name: Option<&'a str>,
    fields: &'a [String],
    tenant: Option<&'a str>,
    scopes: &'a [String],
    output: Option<&'a str>,
    check: bool,
    stdout: bool,
}

fn generate_command(args: GenerateCommandArgs<'_>) {
    match args.gen_type {
        "resource" | "model" => {
            let Some(name) = args.name else {
                eprintln!(
                    "❌ Missing name. Use `rustwing g {} <name>`.",
                    args.gen_type
                );
                std::process::exit(1);
            };
            generate::run(args.gen_type, name, args.fields, args.tenant, args.scopes);
        }
        "openapi" => openapi_export::run(args.output, args.check, args.stdout),
        "client" => {
            let Some(language) = args.name else {
                eprintln!("❌ Missing client target. Use `rustwing g client typescript`.");
                std::process::exit(1);
            };
            if args.check || args.stdout {
                eprintln!("❌ --check and --stdout are only supported by `rustwing g openapi`.");
                std::process::exit(1);
            }
            ts_client::run(language, args.output);
        }
        other => {
            eprintln!(
                "❌ Invalid generator: '{}'. Use resource, model, openapi, or client.",
                other
            );
            std::process::exit(1);
        }
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
        .unwrap_or_else(|e| {
            eprintln!("❌ Failed to run cargo — is it installed? {}", e);
            std::process::exit(1);
        });

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
}
