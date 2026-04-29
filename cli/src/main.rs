mod generate;
mod new;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rustwing", about = "Rustwing CLI - Full-stack Rust SaaS framework")]
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
        Commands::Generate { r#type, name, fields } => generate::run(&r#type, &name, &fields),
    }
}
