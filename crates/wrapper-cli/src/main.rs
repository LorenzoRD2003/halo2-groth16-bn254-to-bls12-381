//! Developer CLI for inspecting and validating the workspace scaffold.

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};
use wrapper_backends::BackendRegistry;
use wrapper_circuits::CircuitPlanningView;
use wrapper_core::{ProjectConfig, ProjectStatusReport};

#[derive(Debug, Parser)]
#[command(
  name = "wrapper-cli",
  version,
  about = "Developer tooling for the Halo2 wrapper workspace scaffold",
  long_about = "Developer tooling for the Halo2 wrapper workspace scaffold. This binary reports repository structure, validates configuration, and explains what is intentionally not implemented yet."
)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
  /// Report what the repository currently scaffolds and what is still missing.
  Doctor,
  /// Explain the benchmark layout and how to run placeholder Criterion benches.
  BenchInfo,
  /// Print the current placeholder layout for the future wrapper circuit.
  PrintLayout,
  /// Validate a TOML configuration file against the current scaffold model.
  ValidateConfig {
    /// Path to a TOML config file.
    #[arg(long)]
    config: PathBuf,
  },
  /// Print project purpose, boundaries, and current phase.
  About,
}

fn main() -> Result<()> {
  init_tracing()?;
  let cli = Cli::parse();

  match cli.command {
    Commands::Doctor => run_doctor(),
    Commands::BenchInfo => run_bench_info(),
    Commands::PrintLayout => run_print_layout(),
    Commands::ValidateConfig { config } => run_validate_config(&config)?,
    Commands::About => run_about(),
  }

  Ok(())
}

fn init_tracing() -> Result<()> {
  fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .with_target(false)
    .try_init()
    .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))
}

fn run_doctor() {
  info!("running doctor command");
  let report = ProjectStatusReport::scaffold();
  let registry = BackendRegistry::scaffold();

  println!("Phase: {:?}", report.phase);
  println!("Capabilities:");
  for (capability, status) in report.capabilities.entries {
    println!("  - {capability:?}: {status:?}");
  }

  println!("Limitations:");
  for limitation in report.limitations {
    println!("  - {limitation}");
  }

  println!("Registered backend placeholders:");
  for entry in registry.entries() {
    println!("  - {}: {}", entry.id, entry.description);
  }
}

fn run_bench_info() {
  info!("printing benchmark guidance");
  println!("Benchmark runner: Criterion");
  println!("Command: cargo bench");
  println!("Current placeholder structure:");
  println!("  - crates/wrapper-tests/benches/field/");
  println!("  - crates/wrapper-tests/benches/ecc/");
  println!("  - crates/wrapper-tests/benches/pairing/");
  println!("Current benchmark entry points:");
  println!("  - bench_placeholder_fp");
  println!("  - bench_placeholder_ecc");
  println!("  - bench_placeholder_pairing");
  println!("Warning: all current benchmarks are placeholders only.");
}

fn run_print_layout() {
  info!("printing scaffold layout");
  let config = ProjectConfig::default();
  let view = CircuitPlanningView::from_config(config);
  let layout = view.describe();

  println!("Layout: {}", layout.name);
  for node in layout.nodes {
    println!("  - {} [{}]", node.title, node.id);
  }
}

fn run_validate_config(path: &PathBuf) -> Result<()> {
  info!("validating config at {}", path.display());
  let raw = fs::read_to_string(path)
    .with_context(|| format!("failed to read config file at {}", path.display()))?;
  let config = ProjectConfig::from_toml_str(&raw)?;
  let json = serde_json::to_string_pretty(&config).context("failed to render config as JSON")?;

  println!("Config is valid for the current scaffold.");
  println!("{json}");

  Ok(())
}

fn run_about() {
  info!("printing project overview");
  println!("Project: Halo2 wrapper workspace skeleton");
  println!("Phase: initialization");
  println!("Purpose: scaffold a serious multi-crate codebase for future Halo2 wrapper work.");
  println!("Current implementation: architecture, docs, config models, CLI, placeholders.");
  println!(
    "Not implemented: field arithmetic, ECC, pairings, Groth16 verification, cryptographic circuits."
  );
}
