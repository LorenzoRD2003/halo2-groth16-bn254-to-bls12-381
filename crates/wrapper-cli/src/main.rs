//! Developer CLI for inspecting and validating the current workspace state.
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::trivially_copy_pass_by_ref)]

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};
use wrapper_backends::{
  ArtifactSetLoader, BackendRegistry, Groth16Bn254ArtifactBundle, MidnightDirectOuterBackend,
  OuterCircuitInputArtifacts, OuterProofBackend, SnarkjsGroth16Bn254ArtifactSetLoader,
  parse_snarkjs_groth16_bn254_bundle_with_names,
};
use wrapper_circuits::{
  CircuitPlanningView, LayoutMetrics, PAIRING_TERM_PROFILE_COUNTS, PUBLIC_INPUT_PROFILE_COUNTS,
  PrimitiveCostEntry, PrimitiveCostLayer, PrimitiveCostTable,
  groth16_fixture_ic_accumulator_layout_metrics, groth16_fixture_verifier_layout_metrics,
  groth16_pairing_block_final_exponentiation_easy_part_layout_metrics,
  groth16_pairing_block_final_exponentiation_hard_part_layout_metrics,
  groth16_pairing_block_final_exponentiation_layout_metrics,
  groth16_pairing_block_miller_loop_layout_metrics,
  groth16_pairing_block_pairing_check_groth16_style_layout_metrics,
  groth16_pairing_block_pairing_check_layout_metrics, groth16_pairing_term_count_layout_metrics,
  groth16_public_input_count_layout_metrics, measure_native_circuit_layout,
  outer_wrapper_fixture_layout_metrics, primitive_definitions,
};
use wrapper_core::{
  ProjectConfig, ProjectStatusReport, WrapperExecutionPackage, WrapperExecutionResult, WrapperJob,
};

const SEMAPHORE_PROFILE_PUBLIC_INPUT_NAMES: [&str; 4] =
  ["merkle_root", "nullifier", "message_hash", "scope_hash"];

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
  /// Parse and inspect a `snarkjs` Groth16 BN254 artifact bundle.
  InspectGroth16Bundle {
    /// Logical identifier for the artifact set.
    #[arg(long, default_value = "artifact-bundle")]
    id: String,
    /// Path to `proof.json`.
    #[arg(long)]
    proof: PathBuf,
    /// Path to `public.json`.
    #[arg(long)]
    public: PathBuf,
    /// Path to `verification_key.json`.
    #[arg(long)]
    vk: PathBuf,
    /// Optional semantic names for the ordered public-input vector.
    #[arg(long = "public-input-name")]
    public_input_names: Vec<String>,
  },
  /// Build and print a wrapper-job plan from a `snarkjs` Groth16 BN254 bundle.
  PlanWrapperJob {
    /// Logical identifier for the artifact set / job.
    #[arg(long, default_value = "wrapper-job")]
    id: String,
    /// Path to `proof.json`.
    #[arg(long)]
    proof: PathBuf,
    /// Path to `public.json`.
    #[arg(long)]
    public: PathBuf,
    /// Path to `verification_key.json`.
    #[arg(long)]
    vk: PathBuf,
    /// Optional semantic names for the ordered public-input vector.
    #[arg(long = "public-input-name")]
    public_input_names: Vec<String>,
  },
  /// Export a planned wrapper job as stable JSON.
  ExportWrapperJob {
    /// Logical identifier for the artifact set / job.
    #[arg(long, default_value = "wrapper-job")]
    id: String,
    /// Path to `proof.json`.
    #[arg(long)]
    proof: PathBuf,
    /// Path to `public.json`.
    #[arg(long)]
    public: PathBuf,
    /// Path to `verification_key.json`.
    #[arg(long)]
    vk: PathBuf,
    /// Optional semantic names for the ordered public-input vector.
    #[arg(long = "public-input-name")]
    public_input_names: Vec<String>,
    /// Optional output path for the JSON manifest. Prints to stdout if omitted.
    #[arg(long)]
    output: Option<PathBuf>,
  },
  /// Export a serializable wrapper execution package as stable JSON.
  ExportWrapperPackage {
    /// Logical identifier for the artifact set / package.
    #[arg(long, default_value = "wrapper-package")]
    id: String,
    /// Path to `proof.json`.
    #[arg(long)]
    proof: PathBuf,
    /// Path to `public.json`.
    #[arg(long)]
    public: PathBuf,
    /// Path to `verification_key.json`.
    #[arg(long)]
    vk: PathBuf,
    /// Optional semantic names for the ordered public-input vector.
    #[arg(long = "public-input-name")]
    public_input_names: Vec<String>,
    /// Optional output path for the JSON package. Prints to stdout if omitted.
    #[arg(long)]
    output: Option<PathBuf>,
  },
  /// Run the current stub wrapper executor over a package derived from artifacts.
  ExecuteWrapperStub {
    /// Logical identifier for the artifact set / package.
    #[arg(long, default_value = "wrapper-package")]
    id: String,
    /// Path to `proof.json`.
    #[arg(long)]
    proof: PathBuf,
    /// Path to `public.json`.
    #[arg(long)]
    public: PathBuf,
    /// Path to `verification_key.json`.
    #[arg(long)]
    vk: PathBuf,
    /// Optional semantic names for the ordered public-input vector.
    #[arg(long = "public-input-name")]
    public_input_names: Vec<String>,
    /// Optional output path for the JSON execution result. Prints to stdout if omitted.
    #[arg(long)]
    output: Option<PathBuf>,
  },
  /// Run the real direct Halo2/Midnight wrapper path over a package derived from artifacts.
  ExecuteWrapperDirect {
    /// Logical identifier for the artifact set / package.
    #[arg(long, default_value = "wrapper-package")]
    id: String,
    /// Path to `proof.json`.
    #[arg(long)]
    proof: PathBuf,
    /// Path to `public.json`.
    #[arg(long)]
    public: PathBuf,
    /// Path to `verification_key.json`.
    #[arg(long)]
    vk: PathBuf,
    /// Optional semantic names for the ordered public-input vector.
    #[arg(long = "public-input-name")]
    public_input_names: Vec<String>,
    /// Optional output path for the JSON execution result. Prints to stdout if omitted.
    #[arg(long)]
    output: Option<PathBuf>,
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
    Commands::InspectGroth16Bundle { id, proof, public, vk, public_input_names } => {
      run_inspect_groth16_bundle(&id, &proof, &public, &vk, &public_input_names)?;
    }
    Commands::PlanWrapperJob { id, proof, public, vk, public_input_names } => {
      run_plan_wrapper_job(&id, &proof, &public, &vk, &public_input_names)?;
    }
    Commands::ExportWrapperJob { id, proof, public, vk, public_input_names, output } => {
      run_export_wrapper_job(&id, &proof, &public, &vk, &public_input_names, output.as_ref())?;
    }
    Commands::ExportWrapperPackage { id, proof, public, vk, public_input_names, output } => {
      run_export_wrapper_package(&id, &proof, &public, &vk, &public_input_names, output.as_ref())?;
    }
    Commands::ExecuteWrapperStub { id, proof, public, vk, public_input_names, output } => {
      run_execute_wrapper_stub(&id, &proof, &public, &vk, &public_input_names, output.as_ref())?;
    }
    Commands::ExecuteWrapperDirect { id, proof, public, vk, public_input_names, output } => {
      run_execute_wrapper_direct(&id, &proof, &public, &vk, &public_input_names, output.as_ref())?;
    }
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
  Outer,
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

#[derive(Clone, Debug, Serialize)]
struct DirectWrapperExecutionResult {
  job_id: String,
  backend: String,
  setup_verification_key: wrapper_core::ProducedOuterVerificationKeyJson,
  produced_bundle: wrapper_core::ProducedOuterProofArtifactBundle,
  verification_ok: bool,
  notes: Vec<String>,
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
        id: "bn254_pairing_check_groth16_style".to_owned(),
        label: "bn254 pairing check groth16-style (1 variable + 3 prepared)",
        term_count: Some(4),
        public_input_count: Some(1),
        layout: groth16_pairing_block_pairing_check_groth16_style_layout_metrics(),
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

  if matches!(family, ProfileFamily::All | ProfileFamily::Outer) {
    rows.extend([
      LayoutProfileRow {
        family: "outer",
        id: "outer_wrapper_fixture_total".to_owned(),
        label: "outer wrapper fixture total",
        term_count: Some(4),
        public_input_count: Some(1),
        layout: outer_wrapper_fixture_layout_metrics(),
      },
      semaphore_outer_end_to_end_layout_row(),
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

fn semaphore_outer_end_to_end_layout_row() -> LayoutProfileRow {
  let bundle = parse_snarkjs_groth16_bn254_bundle_with_names(
    "semaphore-depth-10",
    include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/proof.json"),
    include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/public.json"),
    include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/verification_key.json"),
    &SEMAPHORE_PROFILE_PUBLIC_INPUT_NAMES,
  )
  .expect("named Semaphore profiling bundle should parse");
  let package = bundle.build_halo2_outer_execution_package();
  let backend = MidnightDirectOuterBackend;
  let circuit = backend
    .build_outer_circuit(
      &package,
      OuterCircuitInputArtifacts::new(
        Some(include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/proof.json")),
        Some(include_bytes!(
          "../../wrapper-tests/fixtures/groth16/semaphore/verification_key.json"
        )),
      ),
    )
    .expect("Semaphore outer profiling circuit should build");

  LayoutProfileRow {
    family: "outer",
    id: "outer_wrapper_semaphore_end_to_end".to_owned(),
    label: "outer wrapper semaphore end-to-end",
    term_count: Some(4),
    public_input_count: Some(4),
    layout: measure_native_circuit_layout(&circuit),
  }
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

fn load_groth16_bundle(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
) -> Result<Groth16Bn254ArtifactBundle> {
  let proof_json = fs::read(proof_path)
    .with_context(|| format!("failed to read proof file at {}", proof_path.display()))?;
  let public_json = fs::read(public_path)
    .with_context(|| format!("failed to read public-input file at {}", public_path.display()))?;
  let verification_key_json = fs::read(vk_path)
    .with_context(|| format!("failed to read verification-key file at {}", vk_path.display()))?;

  if public_input_names.is_empty() {
    let loader = SnarkjsGroth16Bn254ArtifactSetLoader;
    loader
      .load_artifact_set(identifier, &proof_json, &public_json, &verification_key_json)
      .context("failed to parse Groth16 artifact bundle")
  } else {
    let field_names = public_input_names.iter().map(String::as_str).collect::<Vec<_>>();
    parse_snarkjs_groth16_bn254_bundle_with_names(
      identifier,
      &proof_json,
      &public_json,
      &verification_key_json,
      &field_names,
    )
    .context("failed to parse named Groth16 artifact bundle")
  }
}

fn plan_wrapper_job_from_paths(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
) -> Result<WrapperJob> {
  let bundle =
    load_groth16_bundle(identifier, proof_path, public_path, vk_path, public_input_names)?;
  Ok(bundle.plan_halo2_outer_wrapper_job())
}

fn build_wrapper_execution_package_from_paths(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
) -> Result<WrapperExecutionPackage> {
  let bundle =
    load_groth16_bundle(identifier, proof_path, public_path, vk_path, public_input_names)?;
  Ok(bundle.build_halo2_outer_execution_package())
}

fn run_inspect_groth16_bundle(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
) -> Result<()> {
  info!("inspecting groth16 bundle {}", identifier);
  let bundle =
    load_groth16_bundle(identifier, proof_path, public_path, vk_path, public_input_names)?;

  let loader = SnarkjsGroth16Bn254ArtifactSetLoader;
  print_groth16_bundle_summary(&bundle, &loader);
  Ok(())
}

fn print_groth16_bundle_summary(
  bundle: &Groth16Bn254ArtifactBundle,
  loader: &SnarkjsGroth16Bn254ArtifactSetLoader,
) {
  let summary = loader.summary();

  println!("Bundle: {}", bundle.identifier);
  println!("Loader: {}", summary.name);
  println!("Artifact-set loading: {}", summary.artifact_set_loading_available);
  println!("Public inputs: {}", bundle.public_input_count());
  println!("VK IC points: {}", bundle.verification_key.ic.len());
  println!("Named public inputs: {}", bundle.named_public_inputs.is_some());

  if let Some(named) = &bundle.named_public_inputs {
    println!("Public input names:");
    for entry in &named.entries {
      println!("  - {} = {}", entry.name, entry.value);
    }
  } else {
    println!("Public input values:");
    for (index, value) in bundle.public_inputs.iter().enumerate() {
      println!("  - [{index}] {value:?}");
    }
  }
}

fn run_plan_wrapper_job(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
) -> Result<()> {
  info!("planning wrapper job {}", identifier);
  let job =
    plan_wrapper_job_from_paths(identifier, proof_path, public_path, vk_path, public_input_names)?;
  print_wrapper_job_summary(&job);
  Ok(())
}

fn run_export_wrapper_job(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
  output_path: Option<&PathBuf>,
) -> Result<()> {
  info!("exporting wrapper job {}", identifier);
  let job =
    plan_wrapper_job_from_paths(identifier, proof_path, public_path, vk_path, public_input_names)?;
  let manifest =
    serde_json::to_string_pretty(&job).context("failed to serialize wrapper job as JSON")?;

  if let Some(path) = output_path {
    fs::write(path, format!("{manifest}\n"))
      .with_context(|| format!("failed to write wrapper-job manifest to {}", path.display()))?;
    println!("Wrote wrapper job manifest to {}", path.display());
  } else {
    println!("{manifest}");
  }

  Ok(())
}

fn run_export_wrapper_package(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
  output_path: Option<&PathBuf>,
) -> Result<()> {
  info!("exporting wrapper package {}", identifier);
  let package = build_wrapper_execution_package_from_paths(
    identifier,
    proof_path,
    public_path,
    vk_path,
    public_input_names,
  )?;
  let manifest = serde_json::to_string_pretty(&package)
    .context("failed to serialize wrapper package as JSON")?;

  if let Some(path) = output_path {
    fs::write(path, format!("{manifest}\n"))
      .with_context(|| format!("failed to write wrapper package to {}", path.display()))?;
    println!("Wrote wrapper package to {}", path.display());
  } else {
    println!("{manifest}");
  }

  Ok(())
}

fn run_execute_wrapper_stub(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
  output_path: Option<&PathBuf>,
) -> Result<()> {
  info!("running wrapper stub executor {}", identifier);
  let package = build_wrapper_execution_package_from_paths(
    identifier,
    proof_path,
    public_path,
    vk_path,
    public_input_names,
  )?;
  let result = package.execute_stub();
  emit_execution_result(&result, output_path)
}

fn run_execute_wrapper_direct(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
  output_path: Option<&PathBuf>,
) -> Result<()> {
  info!("running direct wrapper executor {}", identifier);
  let package = build_wrapper_execution_package_from_paths(
    identifier,
    proof_path,
    public_path,
    vk_path,
    public_input_names,
  )?;
  let proof_json = fs::read(proof_path)
    .with_context(|| format!("failed to read proof file at {}", proof_path.display()))?;
  let verification_key_json = fs::read(vk_path)
    .with_context(|| format!("failed to read verification-key file at {}", vk_path.display()))?;
  let artifacts = OuterCircuitInputArtifacts::new(
    Some(proof_json.as_slice()),
    Some(verification_key_json.as_slice()),
  );
  let backend = MidnightDirectOuterBackend;
  let setup_verification_key =
    backend.setup(&package, artifacts).context("direct wrapper setup failed")?;
  let produced_bundle =
    backend.prove(&package, artifacts).context("direct wrapper proving failed")?;
  let verification_ok = backend
    .verify(&package, &produced_bundle, artifacts)
    .context("direct wrapper verification failed")?;

  let result = DirectWrapperExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    setup_verification_key,
    produced_bundle,
    verification_ok,
    notes: vec![
      "executed direct halo2/midnight outer wrapper path".to_owned(),
      "result includes setup verification key, produced proof bundle, and backend verification verdict"
        .to_owned(),
    ],
  };
  emit_json(&result, output_path, "direct wrapper execution result")
}

fn emit_execution_result(
  result: &WrapperExecutionResult,
  output_path: Option<&PathBuf>,
) -> Result<()> {
  emit_json(result, output_path, "wrapper execution result")
}

fn emit_json<T: Serialize>(
  value: &T,
  output_path: Option<&PathBuf>,
  artifact_label: &str,
) -> Result<()> {
  let manifest = serde_json::to_string_pretty(value)
    .context(format!("failed to serialize {artifact_label} as JSON"))?;

  if let Some(path) = output_path {
    fs::write(path, format!("{manifest}\n"))
      .with_context(|| format!("failed to write {artifact_label} to {}", path.display()))?;
    println!("Wrote {artifact_label} to {}", path.display());
  } else {
    println!("{manifest}");
  }

  Ok(())
}
fn print_wrapper_job_summary(job: &WrapperJob) {
  println!("Wrapper job: {}", job.identifier);
  println!("Source proof system: {:?}", job.source.kind);
  println!("Source loader: {}", job.source.source);
  println!("Target proof system: {:?}", job.target.kind);
  println!("Target planner: {}", job.target.source);
  println!("Public inputs: {}", job.public_input_count);
  println!("Named public inputs: {}", job.named_public_inputs.is_some());

  if let Some(named) = &job.named_public_inputs {
    println!("Planned public-input fields:");
    for entry in &named.entries {
      println!("  - {} = {}", entry.name, entry.value);
    }
  }

  println!("Notes:");
  for note in &job.notes {
    println!("  - {note}");
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
