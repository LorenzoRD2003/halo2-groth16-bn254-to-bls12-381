//! Developer CLI for inspecting and validating the current workspace state.
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::trivially_copy_pass_by_ref)]

use std::{fs, io::{BufReader, BufWriter}, path::PathBuf, time::Instant};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt};
use wrapper_backends::{
  ArtifactSetLoader, BackendRegistry, Groth16Bn254ArtifactBundle, MidnightDirectOuterBackend,
  MidnightDirectOuterBackendBls12Host, MidnightDirectOuterBackendBn254Host,
  OuterCircuitInputArtifacts, OuterProofBackend, ProducedOuterSetupArtifactBundle,
  SnarkjsGroth16Bn254ArtifactSetLoader,
  parse_snarkjs_groth16_bn254_bundle_with_names,
};
use wrapper_circuits::{
  CircuitPlanningView, LayoutMetrics, OuterHostFlavor, PAIRING_TERM_PROFILE_COUNTS,
  PUBLIC_INPUT_PROFILE_COUNTS, PrimitiveCostEntry, PrimitiveCostLayer, PrimitiveCostTable,
  groth16_fixture_ic_accumulator_layout_metrics, groth16_fixture_verifier_layout_metrics,
  groth16_pairing_block_final_exponentiation_easy_part_layout_metrics,
  groth16_pairing_block_final_exponentiation_hard_part_layout_metrics,
  groth16_pairing_block_final_exponentiation_layout_metrics,
  groth16_pairing_block_miller_loop_layout_metrics,
  groth16_pairing_block_pairing_check_groth16_style_layout_metrics,
  groth16_pairing_block_pairing_check_layout_metrics, groth16_pairing_term_count_layout_metrics,
  groth16_public_input_count_layout_metrics, measure_host_circuit_layout,
  measure_native_circuit_layout, primitive_definitions,
};
use wrapper_core::{
  ProjectConfig, ProjectStatusReport, WrapperExecutionPackage, WrapperExecutionResult, WrapperJob,
};

const SEMAPHORE_PROFILE_PUBLIC_INPUT_NAMES: [&str; 4] =
  ["merkle_root", "nullifier", "message_hash", "scope_hash"];
const CIRCOM_MULTIPLIER2_PROFILE_PUBLIC_INPUT_NAMES: [&str; 1] = ["public_input_0"];

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

const DIRECT_EXECUTION_MEMORY_LIMIT_BYTES: u64 = 24 * 1024 * 1024 * 1024;

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
    /// Optional outer fixture filter when profiling the `outer` family.
    #[arg(long, value_enum)]
    outer_fixture: Option<OuterProfileFixtureArg>,
    /// Optional outer host filter when profiling the `outer` family.
    #[arg(long, value_enum)]
    outer_host: Option<OuterProfileHostArg>,
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
    /// Direct outer backend / host lane to use.
    #[arg(long, value_enum, default_value_t = DirectOuterBackendArg::MidnightBn254Host)]
    backend: DirectOuterBackendArg,
    /// Optional output path for the JSON execution result. Prints to stdout if omitted.
    #[arg(long)]
    output: Option<PathBuf>,
  },
  /// Run only setup for the direct outer lane and persist reusable setup artifacts.
  ExecuteWrapperDirectSetup {
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
    /// Direct outer backend / host lane to use.
    #[arg(long, value_enum, default_value_t = DirectOuterBackendArg::MidnightBn254Host)]
    backend: DirectOuterBackendArg,
    /// Output path for the reusable setup JSON artifact bundle.
    #[arg(long)]
    output: PathBuf,
  },
  /// Produce a real direct outer proof bundle using previously persisted setup artifacts.
  ExecuteWrapperDirectProve {
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
    /// Direct outer backend / host lane to use.
    #[arg(long, value_enum, default_value_t = DirectOuterBackendArg::MidnightBn254Host)]
    backend: DirectOuterBackendArg,
    /// Path to the reusable setup JSON artifact bundle produced by `execute-wrapper-direct-setup`.
    #[arg(long)]
    setup: PathBuf,
    /// Output path for the produced direct outer proof bundle JSON.
    #[arg(long)]
    output: PathBuf,
  },
  /// Verify one produced direct outer proof bundle against the selected backend and source artifacts.
  ExecuteWrapperDirectVerify {
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
    /// Direct outer backend / host lane to use.
    #[arg(long, value_enum, default_value_t = DirectOuterBackendArg::MidnightBn254Host)]
    backend: DirectOuterBackendArg,
    /// Path to the produced direct outer proof bundle JSON.
    #[arg(long)]
    bundle: PathBuf,
    /// Optional output path for the verification result JSON. Prints to stdout if omitted.
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
    Commands::ProfileLayout { family, outer_fixture, outer_host } => {
      run_profile_layout(family, outer_fixture, outer_host)
    }
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
    Commands::ExecuteWrapperDirect {
      id,
      proof,
      public,
      vk,
      public_input_names,
      backend,
      output,
    } => {
      apply_direct_execution_memory_limit()?;
      run_execute_wrapper_direct(
        &id,
        &proof,
        &public,
        &vk,
        &public_input_names,
        backend,
        output.as_ref(),
      )?;
    }
    Commands::ExecuteWrapperDirectSetup {
      id,
      proof,
      public,
      vk,
      public_input_names,
      backend,
      output,
    } => {
      apply_direct_execution_memory_limit()?;
      run_execute_wrapper_direct_setup(
        &id,
        &proof,
        &public,
        &vk,
        &public_input_names,
        backend,
        &output,
      )?;
    }
    Commands::ExecuteWrapperDirectProve {
      id,
      proof,
      public,
      vk,
      public_input_names,
      backend,
      setup,
      output,
    } => {
      apply_direct_execution_memory_limit()?;
      run_execute_wrapper_direct_prove(
        &id,
        &proof,
        &public,
        &vk,
        &public_input_names,
        backend,
        &setup,
        &output,
      )?;
    }
    Commands::ExecuteWrapperDirectVerify {
      id,
      proof,
      public,
      vk,
      public_input_names,
      backend,
      bundle,
      output,
    } => {
      apply_direct_execution_memory_limit()?;
      run_execute_wrapper_direct_verify(
        &id,
        &proof,
        &public,
        &vk,
        &public_input_names,
        backend,
        &bundle,
        output.as_ref(),
      )?;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OuterProfileFixtureArg {
  CircomMultiplier2,
  Semaphore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OuterProfileHostArg {
  MidnightBn254,
  MidnightBls12381,
}

#[derive(Clone, Debug)]
struct LayoutProfileRow {
  family: &'static str,
  id: String,
  label: &'static str,
  term_count: Option<usize>,
  public_input_count: Option<usize>,
  parse_elapsed_ms: u128,
  package_elapsed_ms: u128,
  build_circuit_elapsed_ms: u128,
  build_elapsed_ms: u128,
  layout_elapsed_ms: u128,
  elapsed_ms: u128,
  layout: LayoutMetrics,
}

#[derive(Clone, Debug, Serialize)]
struct DirectWrapperExecutionResult {
  job_id: String,
  backend: String,
  outer_host: String,
  setup_elapsed_ms: u128,
  prove_elapsed_ms: u128,
  verify_elapsed_ms: u128,
  elapsed_ms: u128,
  setup_verification_key: wrapper_core::ProducedOuterVerificationKeyJson,
  produced_bundle: wrapper_core::ProducedOuterProofArtifactBundle,
  verification_ok: bool,
  notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct DirectWrapperSetupExecutionResult {
  job_id: String,
  backend: String,
  outer_host: String,
  setup_elapsed_ms: u128,
  setup_bundle: ProducedOuterSetupArtifactBundle,
  notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct DirectWrapperProveExecutionResult {
  job_id: String,
  backend: String,
  outer_host: String,
  prove_elapsed_ms: u128,
  produced_bundle: wrapper_core::ProducedOuterProofArtifactBundle,
  notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct DirectWrapperVerifyExecutionResult {
  job_id: String,
  backend: String,
  outer_host: String,
  verify_elapsed_ms: u128,
  verification_ok: bool,
  notes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum DirectOuterBackendArg {
  MidnightBn254Host,
  MidnightBls12381Host,
}

fn init_tracing() -> Result<()> {
  fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .with_target(false)
    .try_init()
    .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))
}

fn apply_direct_execution_memory_limit() -> Result<()> {
  #[cfg(target_os = "linux")]
  {
    let limit = libc::rlimit {
      rlim_cur: DIRECT_EXECUTION_MEMORY_LIMIT_BYTES,
      rlim_max: DIRECT_EXECUTION_MEMORY_LIMIT_BYTES,
    };
    // Safety: `setrlimit` is called with a valid resource kind and a pointer
    // to a properly initialized `rlimit` struct that lives for the duration of
    // the call.
    let result = unsafe { libc::setrlimit(libc::RLIMIT_AS, &limit) };
    if result != 0 {
      return Err(anyhow::anyhow!(
        "failed to apply 24 GiB process memory limit for direct execution commands: {}",
        std::io::Error::last_os_error()
      ));
    }
  }

  Ok(())
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
  println!("Benchmark preflight timing TSV: kind\\tid\\telapsed_ms");
  println!("Current benchmark structure:");
  for module in ["field", "ecc", "outer"] {
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
  println!("  Outer:");
  for bench_name in [
    "bench_outer_circom_multiplier2_bn254_host",
    "bench_outer_circom_multiplier2_bls12_381_host",
    "bench_outer_semaphore_bn254_host",
    "bench_outer_semaphore_bls12_381_host",
  ] {
    println!("  - {bench_name}");
  }
  println!(
    "Warning: current benchmarks use small Midnight-backed sanity circuits. The Miller-loop, final-exponentiation, and pairing-check entries cover only the current narrow pairing and first Groth16-verifier slice, not a broad verifier framework or production wrapper pipeline."
  );
}

fn run_profile_layout(
  family: ProfileFamily,
  outer_fixture: Option<OuterProfileFixtureArg>,
  outer_host: Option<OuterProfileHostArg>,
) {
  info!("printing Groth16 layout profiles for {:?}", family);
  let rows = layout_profile_rows(family, outer_fixture, outer_host);

  println!(
    "family\tid\tlabel\tterm_count\tpublic_input_count\tparse_elapsed_ms\tpackage_elapsed_ms\tbuild_circuit_elapsed_ms\tbuild_elapsed_ms\tlayout_elapsed_ms\telapsed_ms\trows\tcolumn_queries\tk\ttable_rows\tmax_degree\tadvice_columns\tfixed_columns\tlookups\tpermutations\tpoint_sets"
  );

  for row in rows {
    println!(
      "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
      row.family,
      row.id,
      row.label,
      optional_usize(row.term_count),
      optional_usize(row.public_input_count),
      row.parse_elapsed_ms,
      row.package_elapsed_ms,
      row.build_circuit_elapsed_ms,
      row.build_elapsed_ms,
      row.layout_elapsed_ms,
      row.elapsed_ms,
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

fn layout_profile_rows(
  family: ProfileFamily,
  outer_fixture: Option<OuterProfileFixtureArg>,
  outer_host: Option<OuterProfileHostArg>,
) -> Vec<LayoutProfileRow> {
  let mut rows = Vec::new();

  if matches!(family, ProfileFamily::All | ProfileFamily::Blocks) {
    rows.extend([
      LayoutProfileRow {
        family: "blocks",
        id: "bn254_miller_loop_narrow".to_owned(),
        label: "bn254 miller loop narrow",
        term_count: None,
        public_input_count: None,
        ..timed_layout_profile_row(groth16_pairing_block_miller_loop_layout_metrics)
      },
      LayoutProfileRow {
        family: "blocks",
        id: "bn254_final_exponentiation_easy_part".to_owned(),
        label: "bn254 final exponentiation easy part",
        term_count: None,
        public_input_count: None,
        ..timed_layout_profile_row(
          groth16_pairing_block_final_exponentiation_easy_part_layout_metrics,
        )
      },
      LayoutProfileRow {
        family: "blocks",
        id: "bn254_final_exponentiation_hard_part".to_owned(),
        label: "bn254 final exponentiation hard part",
        term_count: None,
        public_input_count: None,
        ..timed_layout_profile_row(
          groth16_pairing_block_final_exponentiation_hard_part_layout_metrics,
        )
      },
      LayoutProfileRow {
        family: "blocks",
        id: "bn254_final_exponentiation".to_owned(),
        label: "bn254 final exponentiation",
        term_count: None,
        public_input_count: None,
        ..timed_layout_profile_row(groth16_pairing_block_final_exponentiation_layout_metrics)
      },
      LayoutProfileRow {
        family: "blocks",
        id: "bn254_pairing_check_groth16_style".to_owned(),
        label: "bn254 pairing check groth16-style (1 variable + 3 prepared)",
        term_count: Some(4),
        public_input_count: Some(1),
        ..timed_layout_profile_row(groth16_pairing_block_pairing_check_groth16_style_layout_metrics)
      },
      LayoutProfileRow {
        family: "blocks",
        id: "bn254_pairing_check_sample_2_terms".to_owned(),
        label: "bn254 pairing check sample",
        term_count: Some(2),
        public_input_count: None,
        ..timed_layout_profile_row(groth16_pairing_block_pairing_check_layout_metrics)
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
        ..timed_layout_profile_row(groth16_fixture_verifier_layout_metrics)
      },
      LayoutProfileRow {
        family: "groth16",
        id: "groth16_fixture_vk_x_accumulator".to_owned(),
        label: "groth16 fixture vk_x accumulator",
        term_count: None,
        public_input_count: Some(1),
        ..timed_layout_profile_row(groth16_fixture_ic_accumulator_layout_metrics)
      },
      LayoutProfileRow {
        family: "groth16",
        id: "groth16_pairing_check_proxy_4_terms".to_owned(),
        label: "groth16 pairing check 4-term proxy",
        term_count: Some(4),
        public_input_count: None,
        ..timed_layout_profile_row(|| groth16_pairing_term_count_layout_metrics(4))
      },
    ]);
  }

  if matches!(family, ProfileFamily::All | ProfileFamily::Outer) {
    let fixture_filter =
      |fixture: OuterProfileFixtureArg| outer_fixture.map_or(true, |selected| selected == fixture);
    let host_filter = |host: OuterHostFlavor| match outer_host {
      None => true,
      Some(OuterProfileHostArg::MidnightBn254) => host == OuterHostFlavor::MidnightBn254,
      Some(OuterProfileHostArg::MidnightBls12381) => host == OuterHostFlavor::MidnightBls12_381,
    };

    for host in [OuterHostFlavor::MidnightBn254, OuterHostFlavor::MidnightBls12_381] {
      if fixture_filter(OuterProfileFixtureArg::CircomMultiplier2) && host_filter(host) {
        rows.push(outer_fixture_end_to_end_layout_row(
          "circom-multiplier2",
          "circom_multiplier2",
          "outer wrapper circom_multiplier2 end-to-end",
          include_bytes!("../../wrapper-tests/fixtures/groth16/circom_multiplier2/proof.json"),
          include_bytes!("../../wrapper-tests/fixtures/groth16/circom_multiplier2/public.json"),
          include_bytes!(
            "../../wrapper-tests/fixtures/groth16/circom_multiplier2/verification_key.json"
          ),
          &CIRCOM_MULTIPLIER2_PROFILE_PUBLIC_INPUT_NAMES,
          host,
        ));
      }

      if fixture_filter(OuterProfileFixtureArg::Semaphore) && host_filter(host) {
        rows.push(outer_fixture_end_to_end_layout_row(
          "semaphore-depth-10",
          "semaphore",
          "outer wrapper semaphore end-to-end",
          include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/proof.json"),
          include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/public.json"),
          include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/verification_key.json"),
          &SEMAPHORE_PROFILE_PUBLIC_INPUT_NAMES,
          host,
        ));
      }
    }
  }

  if matches!(family, ProfileFamily::All | ProfileFamily::PairingTerms) {
    rows.extend(PAIRING_TERM_PROFILE_COUNTS.iter().map(|count| LayoutProfileRow {
      family: "pairing_terms",
      id: format!("pairing_check_terms_{count}"),
      label: "pairing check term scaling",
      term_count: Some(*count),
      public_input_count: None,
      ..timed_layout_profile_row(|| groth16_pairing_term_count_layout_metrics(*count))
    }));
  }

  if matches!(family, ProfileFamily::All | ProfileFamily::PublicInputs) {
    rows.extend(PUBLIC_INPUT_PROFILE_COUNTS.iter().map(|count| LayoutProfileRow {
      family: "public_inputs",
      id: format!("groth16_ic_accumulator_public_inputs_{count}"),
      label: "groth16 ic accumulator public-input scaling",
      term_count: None,
      public_input_count: Some(*count),
      ..timed_layout_profile_row(|| groth16_public_input_count_layout_metrics(*count))
    }));
  }

  rows
}

fn outer_fixture_end_to_end_layout_row(
  artifact_id: &str,
  fixture_slug: &str,
  fixture_label: &'static str,
  proof_json: &'static [u8],
  public_json: &'static [u8],
  verification_key_json: &'static [u8],
  public_input_names: &[&str],
  outer_host: OuterHostFlavor,
) -> LayoutProfileRow {
  let host_suffix = match outer_host {
    OuterHostFlavor::MidnightBn254 => "bn254_host",
    OuterHostFlavor::MidnightBls12_381 => "bls12_381_host",
  };
  let host_label = match outer_host {
    OuterHostFlavor::MidnightBn254 => "bn254 host",
    OuterHostFlavor::MidnightBls12_381 => "bls12-381 host",
  };
  let parse_started_at = Instant::now();
  let bundle = parse_snarkjs_groth16_bn254_bundle_with_names(
    artifact_id,
    proof_json,
    public_json,
    verification_key_json,
    public_input_names,
  )
  .expect("named outer profiling bundle should parse");
  let parse_elapsed_ms = parse_started_at.elapsed().as_millis();
  let package_started_at = Instant::now();
  let package = bundle.build_halo2_outer_execution_package();
  let package_elapsed_ms = package_started_at.elapsed().as_millis();
  let build_started_at = Instant::now();
  let circuit = match outer_host {
    OuterHostFlavor::MidnightBn254 => {
      let backend = MidnightDirectOuterBackend;
      backend
        .build_outer_circuit(
          &package,
          OuterCircuitInputArtifacts::new(Some(proof_json), Some(verification_key_json)),
        )
        .expect("BN254 outer profiling circuit should build")
    }
    OuterHostFlavor::MidnightBls12_381 => {
      let backend = MidnightDirectOuterBackendBls12Host;
      backend
        .build_outer_circuit(
          &package,
          OuterCircuitInputArtifacts::new(Some(proof_json), Some(verification_key_json)),
        )
        .expect("BLS12 outer profiling circuit should build")
    }
  };
  let build_circuit_elapsed_ms = build_started_at.elapsed().as_millis();
  let build_elapsed_ms = parse_elapsed_ms + package_elapsed_ms + build_circuit_elapsed_ms;
  let layout_started_at = Instant::now();
  let layout = match outer_host {
    OuterHostFlavor::MidnightBn254 => measure_native_circuit_layout(&circuit.hosted_bn254()),
    OuterHostFlavor::MidnightBls12_381 => measure_host_circuit_layout(&circuit.hosted_bls12()),
  };
  let layout_elapsed_ms = layout_started_at.elapsed().as_millis();
  let label = Box::leak(format!("{fixture_label} ({host_label})").into_boxed_str());

  LayoutProfileRow {
    family: "outer",
    id: format!("outer_wrapper_{fixture_slug}_end_to_end_{host_suffix}"),
    label,
    term_count: Some(4),
    public_input_count: Some(public_input_names.len()),
    parse_elapsed_ms,
    package_elapsed_ms,
    build_circuit_elapsed_ms,
    build_elapsed_ms,
    layout_elapsed_ms,
    elapsed_ms: build_elapsed_ms + layout_elapsed_ms,
    layout,
  }
}

fn timed_layout_profile_row(measure: impl FnOnce() -> LayoutMetrics) -> LayoutProfileRow {
  let started_at = Instant::now();
  let layout = measure();
  let layout_elapsed_ms = started_at.elapsed().as_millis();
  LayoutProfileRow {
    family: "",
    id: String::new(),
    label: "",
    term_count: None,
    public_input_count: None,
    parse_elapsed_ms: 0,
    package_elapsed_ms: 0,
    build_circuit_elapsed_ms: 0,
    build_elapsed_ms: 0,
    layout_elapsed_ms,
    elapsed_ms: layout_elapsed_ms,
    layout,
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
  backend_arg: DirectOuterBackendArg,
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
  let result = match backend_arg {
    DirectOuterBackendArg::MidnightBn254Host => execute_wrapper_direct_with_backend(
      &MidnightDirectOuterBackendBn254Host,
      &package,
      artifacts,
    )?,
    DirectOuterBackendArg::MidnightBls12381Host => execute_wrapper_direct_with_backend(
      &MidnightDirectOuterBackendBls12Host,
      &package,
      artifacts,
    )?,
  };
  emit_json(&result, output_path, "direct wrapper execution result")
}

fn run_execute_wrapper_direct_setup(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
  backend_arg: DirectOuterBackendArg,
  output_path: &PathBuf,
) -> Result<()> {
  info!("running direct wrapper setup {}", identifier);
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
  let result = match backend_arg {
    DirectOuterBackendArg::MidnightBn254Host => {
      execute_wrapper_direct_setup_with_bn254_backend(
        &MidnightDirectOuterBackendBn254Host,
        &package,
        artifacts,
        output_path,
      )?
    }
    DirectOuterBackendArg::MidnightBls12381Host => {
      execute_wrapper_direct_setup_with_bls12_backend(
        &MidnightDirectOuterBackendBls12Host,
        &package,
        artifacts,
        output_path,
      )?
    }
  };
  emit_json(&result, Some(output_path), "direct wrapper setup artifact bundle")
}

fn run_execute_wrapper_direct_prove(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
  backend_arg: DirectOuterBackendArg,
  setup_path: &PathBuf,
  output_path: &PathBuf,
) -> Result<()> {
  info!("running direct wrapper proving {}", identifier);
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
  let setup_bundle_json = fs::read(setup_path)
    .with_context(|| format!("failed to read setup bundle file at {}", setup_path.display()))?;
  let setup_bundle: ProducedOuterSetupArtifactBundle = serde_json::from_slice(&setup_bundle_json)
    .with_context(|| format!("failed to parse setup bundle at {}", setup_path.display()))?;
  let artifacts = OuterCircuitInputArtifacts::new(
    Some(proof_json.as_slice()),
    Some(verification_key_json.as_slice()),
  );
  let result = match backend_arg {
    DirectOuterBackendArg::MidnightBn254Host => {
      execute_wrapper_direct_prove_with_bn254_backend(
        &MidnightDirectOuterBackendBn254Host,
        &package,
        artifacts,
        setup_path,
        &setup_bundle,
      )?
    }
    DirectOuterBackendArg::MidnightBls12381Host => {
      execute_wrapper_direct_prove_with_bls12_backend(
        &MidnightDirectOuterBackendBls12Host,
        &package,
        artifacts,
        setup_path,
        &setup_bundle,
      )?
    }
  };
  emit_json(&result, Some(output_path), "direct wrapper produced proof bundle")
}

fn run_execute_wrapper_direct_verify(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
  backend_arg: DirectOuterBackendArg,
  bundle_path: &PathBuf,
  output_path: Option<&PathBuf>,
) -> Result<()> {
  info!("running direct wrapper verification {}", identifier);
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
  let bundle_json = fs::read(bundle_path)
    .with_context(|| format!("failed to read proof bundle file at {}", bundle_path.display()))?;
  let produced_bundle: wrapper_core::ProducedOuterProofArtifactBundle =
    serde_json::from_slice(&bundle_json)
      .with_context(|| format!("failed to parse proof bundle at {}", bundle_path.display()))?;
  let artifacts = OuterCircuitInputArtifacts::new(
    Some(proof_json.as_slice()),
    Some(verification_key_json.as_slice()),
  );
  let result = match backend_arg {
    DirectOuterBackendArg::MidnightBn254Host => {
      execute_wrapper_direct_verify_with_backend(
        &MidnightDirectOuterBackendBn254Host,
        &package,
        artifacts,
        &produced_bundle,
      )?
    }
    DirectOuterBackendArg::MidnightBls12381Host => {
      execute_wrapper_direct_verify_with_backend(
        &MidnightDirectOuterBackendBls12Host,
        &package,
        artifacts,
        &produced_bundle,
      )?
    }
  };
  emit_json(&result, output_path, "direct wrapper verification result")
}

fn execute_wrapper_direct_with_backend<B: OuterProofBackend>(
  backend: &B,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
) -> Result<DirectWrapperExecutionResult> {
  let started_at = Instant::now();
  let setup_started_at = Instant::now();
  let setup_verification_key =
    backend.setup(package, artifacts).context("direct wrapper setup failed")?;
  let setup_elapsed_ms = setup_started_at.elapsed().as_millis();
  let prove_started_at = Instant::now();
  let produced_bundle =
    backend.prove(package, artifacts).context("direct wrapper proving failed")?;
  let prove_elapsed_ms = prove_started_at.elapsed().as_millis();
  let verify_started_at = Instant::now();
  let verification_ok = backend
    .verify(package, &produced_bundle, artifacts)
    .context("direct wrapper verification failed")?;
  let verify_elapsed_ms = verify_started_at.elapsed().as_millis();

  Ok(DirectWrapperExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    setup_elapsed_ms,
    prove_elapsed_ms,
    verify_elapsed_ms,
    elapsed_ms: started_at.elapsed().as_millis(),
    setup_verification_key,
    produced_bundle,
    verification_ok,
    notes: vec![
      format!("executed direct outer wrapper path with backend {}", backend.backend_id()),
      format!("selected outer host lane: {}", backend.metadata().outer_host.id()),
      "result includes setup verification key, produced proof bundle, and backend verification verdict"
        .to_owned(),
    ],
  })
}

fn execute_wrapper_direct_setup_with_bn254_backend(
  backend: &MidnightDirectOuterBackendBn254Host,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  output_path: &PathBuf,
) -> Result<DirectWrapperSetupExecutionResult> {
  let started_at = Instant::now();
  let circuit = backend
    .build_outer_circuit(package, artifacts)
    .context("direct wrapper setup failed while building outer circuit")?;
  let proving_key_artifact = proving_key_sidecar_path(output_path);
  let proving_key_file = fs::File::create(&proving_key_artifact).with_context(|| {
    format!(
      "failed to create proving-key sidecar file at {}",
      proving_key_artifact.display()
    )
  })?;
  let mut proving_key_writer = BufWriter::new(proving_key_file);
  let setup_bundle = backend
    .write_setup_bundle(
      package,
      &circuit,
      &mut proving_key_writer,
      proving_key_artifact
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("direct-setup.pk")
        .to_owned(),
    )
    .context("direct wrapper setup failed while producing reusable setup bundle")?;

  Ok(DirectWrapperSetupExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    setup_elapsed_ms: started_at.elapsed().as_millis(),
    setup_bundle,
    notes: vec![
      format!("executed direct outer setup path with backend {}", backend.backend_id()),
      "setup result contains reusable proving key material so later prove runs can skip setup"
        .to_owned(),
    ],
  })
}

fn execute_wrapper_direct_setup_with_bls12_backend(
  backend: &MidnightDirectOuterBackendBls12Host,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  output_path: &PathBuf,
) -> Result<DirectWrapperSetupExecutionResult> {
  let started_at = Instant::now();
  let circuit = backend
    .build_outer_circuit(package, artifacts)
    .context("direct wrapper setup failed while building outer circuit")?;
  let proving_key_artifact = proving_key_sidecar_path(output_path);
  let proving_key_file = fs::File::create(&proving_key_artifact).with_context(|| {
    format!(
      "failed to create proving-key sidecar file at {}",
      proving_key_artifact.display()
    )
  })?;
  let mut proving_key_writer = BufWriter::new(proving_key_file);
  let setup_bundle = backend
    .write_setup_bundle(
      package,
      &circuit,
      &mut proving_key_writer,
      proving_key_artifact
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("direct-setup.pk")
        .to_owned(),
    )
    .context("direct wrapper setup failed while producing reusable setup bundle")?;

  Ok(DirectWrapperSetupExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    setup_elapsed_ms: started_at.elapsed().as_millis(),
    setup_bundle,
    notes: vec![
      format!("executed direct outer setup path with backend {}", backend.backend_id()),
      "setup result contains reusable proving key material so later prove runs can skip setup"
        .to_owned(),
    ],
  })
}

fn execute_wrapper_direct_prove_with_bn254_backend(
  backend: &MidnightDirectOuterBackendBn254Host,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  setup_path: &PathBuf,
  setup_bundle: &ProducedOuterSetupArtifactBundle,
) -> Result<DirectWrapperProveExecutionResult> {
  let started_at = Instant::now();
  let circuit = backend
    .build_outer_circuit(package, artifacts)
    .context("direct wrapper prove failed while building outer circuit")?;
  let proving_key_path = resolve_setup_sidecar_path(setup_path, setup_bundle);
  let proving_key_file = fs::File::open(&proving_key_path).with_context(|| {
    format!("failed to open proving-key sidecar file at {}", proving_key_path.display())
  })?;
  let mut proving_key_reader = BufReader::new(proving_key_file);
  let produced_bundle = backend
    .produce_proof_bundle_from_setup_reader(package, &circuit, setup_bundle, &mut proving_key_reader)
    .context("direct wrapper prove failed while reusing setup bundle")?;

  Ok(DirectWrapperProveExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    prove_elapsed_ms: started_at.elapsed().as_millis(),
    produced_bundle,
    notes: vec![
      format!("executed direct outer prove path with backend {}", backend.backend_id()),
      "prove reused a persisted setup bundle instead of rerunning setup".to_owned(),
    ],
  })
}

fn execute_wrapper_direct_prove_with_bls12_backend(
  backend: &MidnightDirectOuterBackendBls12Host,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  setup_path: &PathBuf,
  setup_bundle: &ProducedOuterSetupArtifactBundle,
) -> Result<DirectWrapperProveExecutionResult> {
  let started_at = Instant::now();
  let circuit = backend
    .build_outer_circuit(package, artifacts)
    .context("direct wrapper prove failed while building outer circuit")?;
  let proving_key_path = resolve_setup_sidecar_path(setup_path, setup_bundle);
  let proving_key_file = fs::File::open(&proving_key_path).with_context(|| {
    format!("failed to open proving-key sidecar file at {}", proving_key_path.display())
  })?;
  let mut proving_key_reader = BufReader::new(proving_key_file);
  let produced_bundle = backend
    .produce_proof_bundle_from_setup_reader(package, &circuit, setup_bundle, &mut proving_key_reader)
    .context("direct wrapper prove failed while reusing setup bundle")?;

  Ok(DirectWrapperProveExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    prove_elapsed_ms: started_at.elapsed().as_millis(),
    produced_bundle,
    notes: vec![
      format!("executed direct outer prove path with backend {}", backend.backend_id()),
      "prove reused a persisted setup bundle instead of rerunning setup".to_owned(),
    ],
  })
}

fn execute_wrapper_direct_verify_with_backend<B: OuterProofBackend>(
  backend: &B,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  produced_bundle: &wrapper_core::ProducedOuterProofArtifactBundle,
) -> Result<DirectWrapperVerifyExecutionResult> {
  let started_at = Instant::now();
  let verification_ok = backend
    .verify(package, produced_bundle, artifacts)
    .context("direct wrapper verification failed")?;

  Ok(DirectWrapperVerifyExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    verify_elapsed_ms: started_at.elapsed().as_millis(),
    verification_ok,
    notes: vec![
      format!("executed direct outer verify path with backend {}", backend.backend_id()),
      "verify checked a previously produced proof bundle without rerunning setup or prove"
        .to_owned(),
    ],
  })
}

fn emit_execution_result(
  result: &WrapperExecutionResult,
  output_path: Option<&PathBuf>,
) -> Result<()> {
  emit_json(result, output_path, "wrapper execution result")
}

fn proving_key_sidecar_path(setup_manifest_path: &PathBuf) -> PathBuf {
  let mut file_name = setup_manifest_path
    .file_name()
    .and_then(|name| name.to_str())
    .unwrap_or("direct-setup.json")
    .to_owned();
  file_name.push_str(".pk");
  setup_manifest_path.with_file_name(file_name)
}

fn resolve_setup_sidecar_path(
  setup_manifest_path: &PathBuf,
  setup_bundle: &ProducedOuterSetupArtifactBundle,
) -> PathBuf {
  setup_manifest_path.with_file_name(&setup_bundle.proving_key.proving_key_artifact)
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
