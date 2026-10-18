use clap::{Parser, Subcommand, ValueEnum};
use crashledger_engine::{run_demo, DemoMode};
use crashledger_evidence::read_report;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "crashledger",
    version,
    about = "Local-first failure experiments with independent observations"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum Mode {
    Broken,
    Fixed,
}
impl From<Mode> for DemoMode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Broken => DemoMode::Broken,
            Mode::Fixed => DemoMode::Fixed,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum ReportFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
enum Command {
    /// Validate a closed-schema scenario without contacting its target.
    Validate { scenario: PathBuf },
    /// Run the lost-response scenario against a fresh broken or fixed fixture.
    Demo {
        #[arg(long, value_enum)]
        mode: Mode,
        /// Output root for isolated run directories and their fixture databases.
        #[arg(long, default_value = "artifacts")]
        artifacts_dir: PathBuf,
    },
    /// Verify SHA-256, validate the event history and reconstruct a stored verdict.
    Report {
        bundle: PathBuf,
        #[arg(long, value_enum, default_value = "text")]
        format: ReportFormat,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Validate { scenario } => match crashledger_scenario::load_file(&scenario) {
            Ok(s) => {
                println!("VALID: {} (schema v{})", s.id, s.schema_version);
                println!("Target: {} [loopback only]", s.target.base_url);
                println!(
                    "Attempts: {}; fault fires: {}",
                    s.workload.retry.max_attempts, s.fault.max_fires
                );
                0
            }
            Err(error) => {
                eprintln!("ERROR: {error}");
                4
            }
        },
        Command::Demo { mode, artifacts_dir } => match run_demo(mode.into(), &artifacts_dir).await {
            Ok(summary) => {
                summary.print();
                summary.decision.verdict.exit_code()
            }
            Err(error) => {
                eprintln!("ERROR: {error}");
                4
            }
        },
        Command::Report { bundle, format } => match read_report(&bundle) {
            Ok(report) => {
                match format {
                    ReportFormat::Text => print!("{}", report.text()),
                    ReportFormat::Json => match serde_json::to_string_pretty(&report) {
                        Ok(json) => println!("{json}"),
                        Err(error) => {
                            eprintln!("ERROR: cannot format report: {error}");
                            std::process::exit(4);
                        }
                    },
                };
                0
            }
            Err(error) => {
                eprintln!("ERROR: report rejected: {error}");
                4
            }
        },
    };
    std::process::exit(code);
}
