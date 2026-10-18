use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "crashledger", version, about = "Local-first failure experiments with independent observations")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse and validate a closed schema v1 YAML scenario without contacting its target.
    Validate {
        /// Path to a YAML scenario file.
        scenario: PathBuf,
    },
}

fn main() {
    let args = Cli::parse();
    let result = match args.command {
        Command::Validate { scenario } => crashledger_scenario::load_file(&scenario).map(|s| {
            println!("VALID: {} (schema v{})", s.id, s.schema_version);
            println!("Target: {} [loopback only]", s.target.base_url);
            println!(
                "Attempts: {}; fault fires: {}",
                s.workload.retry.max_attempts, s.fault.max_fires
            );
        }),
    };
    if let Err(error) = result {
        eprintln!("ERROR: {error}");
        std::process::exit(4);
    }
}
