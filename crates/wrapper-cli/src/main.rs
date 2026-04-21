//! Developer CLI for inspecting and validating the current workspace state.
#![allow(clippy::multiple_crate_versions)]

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};
use wrapper_backends::BackendRegistry;
use wrapper_circuits::{CircuitPlanningView, CostEstimate, LayoutMetrics, PrimitiveCostTable};
use wrapper_core::{ProjectConfig, ProjectStatusReport};

#[derive(Debug, Parser)]
#[command(
  name = "wrapper-cli",
  version,
  about = "Developer tooling for the Halo2 wrapper workspace",
  long_about = "Developer tooling for the Halo2 wrapper workspace. This binary reports repository structure, validates configuration, and explains what is intentionally not implemented yet."
)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
  /// Report what the repository currently implements and what is still missing.
  Doctor,
  /// Explain the benchmark layout and how to run placeholder Criterion benches.
  BenchInfo,
  /// Print the current placeholder layout for the future wrapper circuit.
  PrintLayout,
  /// Validate a TOML configuration file against the current project model.
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
  let planning = CircuitPlanningView::from_config(ProjectConfig::default());
  let primitive_costs = planning.primitive_cost_table();

  print_doctor_status(report, &registry);
  print_primitive_costs(&primitive_costs);
}

fn print_doctor_status(report: ProjectStatusReport, registry: &BackendRegistry) {
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

fn print_cost_line(name: &str, cost: CostEstimate, layout: LayoutMetrics) {
  println!(
    "  - {name}: {} rows / {} queries (k={}, advice={}, fixed={})",
    cost.rows, cost.constraints, layout.k, layout.advice_columns, layout.fixed_columns
  );
}

fn print_cost_line_with_lookups(name: &str, cost: CostEstimate, layout: LayoutMetrics) {
  println!(
    "  - {name}: {} rows / {} queries (k={}, advice={}, fixed={}, lookups={})",
    cost.rows,
    cost.constraints,
    layout.k,
    layout.advice_columns,
    layout.fixed_columns,
    layout.lookups
  );
}

fn print_primitive_costs(primitive_costs: &PrimitiveCostTable) {
  println!("Week 1 / Week 3 primitive estimates:");
  print_cost_line("fp add", primitive_costs.fp_add, primitive_costs.fp_add_layout);
  print_cost_line("fp mul", primitive_costs.fp_mul, primitive_costs.fp_mul_layout);
  print_cost_line("fp2 add", primitive_costs.fp2_add, primitive_costs.fp2_add_layout);
  print_cost_line("fp2 mul", primitive_costs.fp2_mul, primitive_costs.fp2_mul_layout);
  print_cost_line("fp2 square", primitive_costs.fp2_square, primitive_costs.fp2_square_layout);
  print_cost_line("fp6 add", primitive_costs.fp6_add, primitive_costs.fp6_add_layout);
  print_cost_line("fp6 mul", primitive_costs.fp6_mul, primitive_costs.fp6_mul_layout);
  print_cost_line("fp6 square", primitive_costs.fp6_square, primitive_costs.fp6_square_layout);
  print_cost_line("fp12 add", primitive_costs.fp12_add, primitive_costs.fp12_add_layout);
  print_cost_line("fp12 mul", primitive_costs.fp12_mul, primitive_costs.fp12_mul_layout);
  print_cost_line("fp12 square", primitive_costs.fp12_square, primitive_costs.fp12_square_layout);
  print_cost_line_with_lookups("g1 add", primitive_costs.g1_add, primitive_costs.g1_add_layout);
  print_cost_line("g2 on_curve", primitive_costs.g2_on_curve, primitive_costs.g2_on_curve_layout);
  print_cost_line("g2 neg", primitive_costs.g2_neg, primitive_costs.g2_neg_layout);
  print_cost_line(
    "g2 proj from_affine",
    primitive_costs.g2_proj_from_affine,
    primitive_costs.g2_proj_from_affine_layout,
  );
  print_cost_line(
    "g2 proj double",
    primitive_costs.g2_proj_double,
    primitive_costs.g2_proj_double_layout,
  );
  print_cost_line("g2 proj add", primitive_costs.g2_proj_add, primitive_costs.g2_proj_add_layout);
}

fn run_bench_info() {
  info!("printing benchmark guidance");
  println!("Benchmark runner: Criterion");
  println!("Command: cargo bench");
  println!("Current benchmark structure:");
  println!("  - crates/wrapper-tests/benches/field/");
  println!("  - crates/wrapper-tests/benches/ecc/");
  println!("Current benchmark entry points:");
  println!("  - bench_fp_add");
  println!("  - bench_fp_mul");
  println!("  - bench_fp2_add");
  println!("  - bench_fp2_mul");
  println!("  - bench_fp2_square");
  println!("  - bench_fp6_add");
  println!("  - bench_fp6_mul");
  println!("  - bench_fp6_square");
  println!("  - bench_fp12_add");
  println!("  - bench_fp12_mul");
  println!("  - bench_fp12_square");
  println!("  - bench_g1_add");
  println!("  - bench_g2_on_curve");
  println!("  - bench_g2_neg");
  println!("  - bench_g2_proj_from_affine");
  println!("  - bench_g2_proj_double");
  println!("  - bench_g2_proj_add");
  println!(
    "Warning: current benchmarks use small Midnight-backed sanity circuits and do not cover subgroup checks, pairings, or verifier logic."
  );
}

fn run_print_layout() {
  info!("printing current layout view");
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

  println!("Config is valid for the current project model.");
  println!("{json}");

  Ok(())
}

fn run_about() {
  info!("printing project overview");
  println!("Project: Halo2 wrapper workspace");
  println!("Phase: stage 1 / week 2 to early week 3 (fp2 + fp6 + narrow g2 affine/projective)");
  println!("Purpose: stage a serious multi-crate codebase for Halo2 wrapper research.");
  println!(
    "Current implementation: architecture, docs, config models, Midnight-backed BN254 fp/fp2/fp6 arithmetic, minimal G1 add/on-curve checks, narrow G2 affine assign/on-curve/neg plus projective from_affine/add/double/neg, CLI, and sanity-check benches."
  );
  println!(
    "Not implemented: G2 subgroup checks or scalar multiplication, Fp12, pairings, Groth16 verification, and wrapper verifier circuits."
  );
}
