//! Developer CLI for inspecting and validating the current workspace state.
#![allow(clippy::multiple_crate_versions)]

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};
use wrapper_backends::BackendRegistry;
use wrapper_circuits::{
  CircuitPlanningView, LayoutMetrics, PAIRING_TERM_PROFILE_COUNTS, PUBLIC_INPUT_PROFILE_COUNTS,
  PrimitiveCostEntry, PrimitiveCostLayer, PrimitiveCostTable,
  groth16_fixture_ic_accumulator_layout_metrics, groth16_fixture_verifier_layout_metrics,
  groth16_pairing_block_final_exponentiation_easy_part_layout_metrics,
  groth16_pairing_block_final_exponentiation_hard_part_layout_metrics,
  groth16_pairing_block_final_exponentiation_layout_metrics,
  groth16_pairing_block_miller_loop_layout_metrics,
  groth16_pairing_block_pairing_check_layout_metrics, groth16_pairing_term_count_layout_metrics,
  groth16_public_input_count_layout_metrics, primitive_definitions,
};
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
  /// Emit stable TSV layout profiles for the current Groth16 verifier slice.
  ProfileLayout {
    /// Which measurement family to emit.
    #[arg(long, value_enum, default_value_t = ProfileFamily::All)]
    family: ProfileFamily,
  },
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
    Commands::ProfileLayout { family } => run_profile_layout(family),
    Commands::PrintLayout => run_print_layout(),
    Commands::ValidateConfig { config } => run_validate_config(&config)?,
    Commands::About => run_about(),
  }

  Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ProfileFamily {
  All,
  Blocks,
  Groth16,
  PairingTerms,
  PublicInputs,
}

#[derive(Clone, Debug)]
struct LayoutProfileRow {
  family: &'static str,
  id: String,
  label: &'static str,
  term_count: Option<usize>,
  public_input_count: Option<usize>,
  layout: LayoutMetrics,
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

fn print_cost_line(name: &str, layout: LayoutMetrics) {
  let cost = layout.cost_estimate();
  println!(
    "  - {name}: {} rows / {} queries (k={}, advice={}, fixed={})",
    cost.rows, cost.constraints, layout.k, layout.advice_columns, layout.fixed_columns
  );
}

fn print_cost_line_with_lookups(name: &str, layout: LayoutMetrics) {
  let cost = layout.cost_estimate();
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
  println!("Current Stage 1 / Week 5 primitive estimates:");
  let entries = primitive_costs.entries();
  print_cost_group(PrimitiveCostLayer::Field, entries);
  print_cost_group(PrimitiveCostLayer::Curve, entries);
  print_cost_group(PrimitiveCostLayer::MillerPrep, entries);
  print_cost_group(PrimitiveCostLayer::MillerLoop, entries);
}

fn print_cost_group(layer: PrimitiveCostLayer, entries: &[PrimitiveCostEntry]) {
  println!("  {layer}:");
  for entry in entries.iter().filter(|entry| entry.definition.layer == layer) {
    if entry.definition.show_lookups {
      print_cost_line_with_lookups(entry.definition.label, entry.layout);
    } else {
      print_cost_line(entry.definition.label, entry.layout);
    }
  }
}

fn run_bench_info() {
  info!("printing benchmark guidance");
  println!("Benchmark runner: Criterion");
  println!("Command: cargo bench");
  println!("Current benchmark structure:");
  for module in ["field", "ecc"] {
    println!("  - crates/wrapper-tests/benches/{module}/");
  }
  println!("Current benchmark entry points:");
  for layer in [
    PrimitiveCostLayer::Field,
    PrimitiveCostLayer::Curve,
    PrimitiveCostLayer::MillerPrep,
    PrimitiveCostLayer::MillerLoop,
  ] {
    print_bench_group(layer);
  }
  println!(
    "Warning: current benchmarks use small Midnight-backed sanity circuits. The Miller-loop, final-exponentiation, and pairing-check entries cover only the current narrow pairing and first Groth16-verifier slice, not a broad verifier framework or production wrapper pipeline."
  );
}

fn run_profile_layout(family: ProfileFamily) {
  info!("printing Groth16 layout profiles for {:?}", family);
  let rows = layout_profile_rows(family);

  println!(
    "family\tid\tlabel\tterm_count\tpublic_input_count\trows\tcolumn_queries\tk\ttable_rows\tmax_degree\tadvice_columns\tfixed_columns\tlookups\tpermutations\tpoint_sets"
  );

  for row in rows {
    println!(
      "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
      row.family,
      row.id,
      row.label,
      optional_usize(row.term_count),
      optional_usize(row.public_input_count),
      row.layout.rows,
      row.layout.column_queries,
      row.layout.k,
      row.layout.table_rows,
      row.layout.max_degree,
      row.layout.advice_columns,
      row.layout.fixed_columns,
      row.layout.lookups,
      row.layout.permutations,
      row.layout.point_sets,
    );
  }
}

fn layout_profile_rows(family: ProfileFamily) -> Vec<LayoutProfileRow> {
  let mut rows = Vec::new();

  if matches!(family, ProfileFamily::All | ProfileFamily::Blocks) {
    rows.extend([
      LayoutProfileRow {
        family: "blocks",
        id: "bn254_miller_loop_narrow".to_owned(),
        label: "bn254 miller loop narrow",
        term_count: None,
        public_input_count: None,
        layout: groth16_pairing_block_miller_loop_layout_metrics(),
      },
      LayoutProfileRow {
        family: "blocks",
        id: "bn254_final_exponentiation_easy_part".to_owned(),
        label: "bn254 final exponentiation easy part",
        term_count: None,
        public_input_count: None,
        layout: groth16_pairing_block_final_exponentiation_easy_part_layout_metrics(),
      },
      LayoutProfileRow {
        family: "blocks",
        id: "bn254_final_exponentiation_hard_part".to_owned(),
        label: "bn254 final exponentiation hard part",
        term_count: None,
        public_input_count: None,
        layout: groth16_pairing_block_final_exponentiation_hard_part_layout_metrics(),
      },
      LayoutProfileRow {
        family: "blocks",
        id: "bn254_final_exponentiation".to_owned(),
        label: "bn254 final exponentiation",
        term_count: None,
        public_input_count: None,
        layout: groth16_pairing_block_final_exponentiation_layout_metrics(),
      },
      LayoutProfileRow {
        family: "blocks",
        id: "bn254_pairing_check_sample_2_terms".to_owned(),
        label: "bn254 pairing check sample",
        term_count: Some(2),
        public_input_count: None,
        layout: groth16_pairing_block_pairing_check_layout_metrics(),
      },
    ]);
  }

  if matches!(family, ProfileFamily::All | ProfileFamily::Groth16) {
    rows.extend([
      LayoutProfileRow {
        family: "groth16",
        id: "groth16_fixture_verifier_total".to_owned(),
        label: "groth16 fixture verifier total",
        term_count: None,
        public_input_count: Some(1),
        layout: groth16_fixture_verifier_layout_metrics(),
      },
      LayoutProfileRow {
        family: "groth16",
        id: "groth16_fixture_vk_x_accumulator".to_owned(),
        label: "groth16 fixture vk_x accumulator",
        term_count: None,
        public_input_count: Some(1),
        layout: groth16_fixture_ic_accumulator_layout_metrics(),
      },
      LayoutProfileRow {
        family: "groth16",
        id: "groth16_pairing_check_proxy_4_terms".to_owned(),
        label: "groth16 pairing check 4-term proxy",
        term_count: Some(4),
        public_input_count: None,
        layout: groth16_pairing_term_count_layout_metrics(4),
      },
    ]);
  }

  if matches!(family, ProfileFamily::All | ProfileFamily::PairingTerms) {
    rows.extend(PAIRING_TERM_PROFILE_COUNTS.iter().map(|count| LayoutProfileRow {
      family: "pairing_terms",
      id: format!("pairing_check_terms_{count}"),
      label: "pairing check term scaling",
      term_count: Some(*count),
      public_input_count: None,
      layout: groth16_pairing_term_count_layout_metrics(*count),
    }));
  }

  if matches!(family, ProfileFamily::All | ProfileFamily::PublicInputs) {
    rows.extend(PUBLIC_INPUT_PROFILE_COUNTS.iter().map(|count| LayoutProfileRow {
      family: "public_inputs",
      id: format!("groth16_ic_accumulator_public_inputs_{count}"),
      label: "groth16 ic accumulator public-input scaling",
      term_count: None,
      public_input_count: Some(*count),
      layout: groth16_public_input_count_layout_metrics(*count),
    }));
  }

  rows
}

fn optional_usize(value: Option<usize>) -> String {
  value.map_or_else(String::new, |value| value.to_string())
}

fn print_bench_group(layer: PrimitiveCostLayer) {
  println!("  {layer}:");
  for definition in primitive_definitions().iter().filter(|definition| definition.layer == layer) {
    println!("  - {}", definition.bench_name);
  }
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
  let overview = ProjectStatusReport::overview();
  println!("Project: Halo2 wrapper workspace");
  println!("Phase: {}", overview.phase_label);
  println!("Purpose: {}", overview.purpose);
  println!("Current implementation: {}", overview.current_implementation);
  println!("Not implemented: {}", overview.not_implemented);
}
