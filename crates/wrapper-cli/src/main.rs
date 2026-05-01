//! Developer CLI for inspecting and validating the current workspace state.
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::trivially_copy_pass_by_ref)]

use std::{
  fs,
  io::{BufReader, BufWriter, Write as _},
  path::{Path, PathBuf},
  time::Instant,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use rayon::ThreadPoolBuilder;
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
  Bn254ExpByXChainSearchConfig, Bn254ExpByXChainSearchWeights, CircuitPlanningView, LayoutMetrics,
  OuterHostFlavor, PAIRING_TERM_PROFILE_COUNTS, PUBLIC_INPUT_PROFILE_COUNTS, PrimitiveCostEntry,
  PrimitiveCostLayer, PrimitiveCostTable,
  groth16_fixture_ic_accumulator_layout_metrics, groth16_fixture_verifier_layout_metrics,
  groth16_pairing_block_final_exponentiation_easy_part_layout_metrics,
  groth16_pairing_block_final_exponentiation_hard_part_layout_metrics,
  groth16_pairing_block_final_exponentiation_hard_part_layout_metrics_v1,
  groth16_pairing_block_final_exponentiation_layout_metrics,
  groth16_pairing_block_miller_loop_layout_metrics,
  groth16_pairing_block_pairing_check_groth16_style_layout_metrics,
  groth16_pairing_block_pairing_check_groth16_style_layout_metrics_v1,
  groth16_pairing_block_pairing_check_layout_metrics, groth16_pairing_term_count_layout_metrics,
  groth16_public_input_count_layout_metrics, measure_host_circuit_layout,
  measure_native_circuit_layout, primitive_definitions, retained_bn254_exp_by_x_chain_candidate,
  search_bn254_exp_by_x_candidates, search_bn254_exp_by_x_candidates_with_windows,
  fp12_compressed_cyclotomic_square_block_layout_metrics, fp12_mul_by_unitary_inverse_layout_metrics,
  fp12_mul_layout_metrics,
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
  /// Search for candidate `exp_by_neg_x(...)` chains using a proxy cost model.
  SearchExpByXChain {
    /// Maximum odd window value considered during the search.
    #[arg(long, default_value_t = 127)]
    max_window: u64,
    /// Maximum number of cyclotomic squarings in one step.
    #[arg(long, default_value_t = 12)]
    max_square_count: u8,
    /// Maximum number of signed steps after the starting window.
    #[arg(long, default_value_t = 8)]
    max_steps: usize,
    /// Number of top candidates to print.
    #[arg(long, default_value_t = 10)]
    limit: usize,
    /// Weight profile to use for the search proxy.
    #[arg(long, value_enum, default_value_t = ExpByXWeightProfile::Empirical)]
    weight_profile: ExpByXWeightProfile,
    /// Proxy cost charged per compressed square.
    #[arg(long, default_value_t = 1)]
    square_cost: u64,
    /// Proxy cost charged per positive window multiplication.
    #[arg(long, default_value_t = 11)]
    positive_mul_cost: u64,
    /// Proxy cost charged per negative window multiplication.
    #[arg(long, default_value_t = 10)]
    negative_mul_cost: u64,
    /// Proxy cost charged once per distinct precomputed window.
    #[arg(long, default_value_t = 4)]
    unique_window_cost: u64,
    /// Proxy cost charged for the starting window.
    #[arg(long, default_value_t = 2)]
    start_window_cost: u64,
    /// Restrict the search to one explicit set of odd windows. Repeatable.
    #[arg(long = "allowed-window")]
    allowed_windows: Vec<u64>,
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
  /// Run only the first stage of direct proving and persist an intermediate prover-trace artifact.
  ExecuteWrapperDirectProveTrace {
    #[arg(long, default_value = "wrapper-package")]
    id: String,
    #[arg(long)]
    proof: PathBuf,
    #[arg(long)]
    public: PathBuf,
    #[arg(long)]
    vk: PathBuf,
    #[arg(long = "public-input-name")]
    public_input_names: Vec<String>,
    #[arg(long, value_enum, default_value_t = DirectOuterBackendArg::MidnightBn254Host)]
    backend: DirectOuterBackendArg,
    /// Path to the reusable setup JSON artifact bundle produced by `execute-wrapper-direct-setup`.
    #[arg(long)]
    setup: PathBuf,
    /// Output path for the persisted prover-trace binary artifact.
    #[arg(long)]
    output: PathBuf,
  },
  /// Finalize a direct outer proof bundle from a previously persisted prover-trace artifact.
  ExecuteWrapperDirectProveFinalize {
    #[arg(long, default_value = "wrapper-package")]
    id: String,
    #[arg(long)]
    proof: PathBuf,
    #[arg(long)]
    public: PathBuf,
    #[arg(long)]
    vk: PathBuf,
    #[arg(long = "public-input-name")]
    public_input_names: Vec<String>,
    #[arg(long, value_enum, default_value_t = DirectOuterBackendArg::MidnightBn254Host)]
    backend: DirectOuterBackendArg,
    /// Path to the reusable setup JSON artifact bundle produced by `execute-wrapper-direct-setup`.
    #[arg(long)]
    setup: PathBuf,
    /// Path to the persisted prover-trace binary artifact produced by `execute-wrapper-direct-prove-trace`.
    #[arg(long)]
    trace: PathBuf,
    /// Optional base-2 exponent for the chunked `h_poly` permutation row chunk
    /// size in `prove-finalize`; for example, `16` means `2^16 = 65536` rows.
    #[arg(long)]
    h_poly_row_chunk_size: Option<u32>,
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
      run_profile_layout(family, outer_fixture, outer_host);
    }
    Commands::SearchExpByXChain {
      max_window,
      max_square_count,
      max_steps,
      limit,
      weight_profile,
      square_cost,
      positive_mul_cost,
      negative_mul_cost,
      unique_window_cost,
      start_window_cost,
      allowed_windows,
    } => run_search_exp_by_x_chain(
      ExpByXSearchArgs {
        max_window,
        max_square_count,
        max_steps,
        limit,
        weight_profile,
        square_cost,
        positive_mul_cost,
        negative_mul_cost,
        unique_window_cost,
        start_window_cost,
        allowed_windows: &allowed_windows,
      },
    ),
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
      configure_direct_execution_runtime();
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
      configure_direct_execution_runtime();
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
      configure_direct_execution_runtime();
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
      configure_direct_execution_runtime();
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
    Commands::ExecuteWrapperDirectProveTrace {
      id,
      proof,
      public,
      vk,
      public_input_names,
      backend,
      setup,
      output,
    } => {
      configure_direct_execution_runtime();
      apply_direct_execution_memory_limit()?;
      run_execute_wrapper_direct_prove_trace(
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
    Commands::ExecuteWrapperDirectProveFinalize {
      id,
      proof,
      public,
      vk,
      public_input_names,
      backend,
      setup,
      trace,
      h_poly_row_chunk_size,
      output,
    } => {
      configure_direct_execution_runtime();
      apply_direct_execution_memory_limit()?;
      run_execute_wrapper_direct_prove_finalize(
        DirectProveFinalizeArgs {
          identifier: &id,
          proof_path: &proof,
          public_path: &public,
          vk_path: &vk,
          public_input_names: &public_input_names,
          backend_arg: backend,
          setup_path: &setup,
          trace_path: &trace,
          h_poly_row_chunk_size,
          output_path: &output,
        },
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
  FloorPlanner,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ExpByXWeightProfile {
  Linear,
  Empirical,
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

#[derive(Clone, Copy)]
struct ExpByXSearchArgs<'a> {
  max_window: u64,
  max_square_count: u8,
  max_steps: usize,
  limit: usize,
  weight_profile: ExpByXWeightProfile,
  square_cost: u64,
  positive_mul_cost: u64,
  negative_mul_cost: u64,
  unique_window_cost: u64,
  start_window_cost: u64,
  allowed_windows: &'a [u64],
}

#[derive(Clone, Copy)]
struct DirectProveFinalizeArgs<'a> {
  identifier: &'a str,
  proof_path: &'a PathBuf,
  public_path: &'a PathBuf,
  vk_path: &'a PathBuf,
  public_input_names: &'a [String],
  backend_arg: DirectOuterBackendArg,
  setup_path: &'a PathBuf,
  trace_path: &'a PathBuf,
  h_poly_row_chunk_size: Option<u32>,
  output_path: &'a PathBuf,
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

#[derive(Clone, Debug, serde::Deserialize)]
struct DirectWrapperSetupExecutionResultJson {
  setup_bundle: ProducedOuterSetupArtifactBundle,
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
struct DirectWrapperProveTraceExecutionResult {
  job_id: String,
  backend: String,
  outer_host: String,
  trace_elapsed_ms: u128,
  trace_artifact: String,
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

impl DirectOuterBackendArg {
  fn backend_id_hint(self) -> &'static str {
    match self {
      Self::MidnightBn254Host => "midnight-direct-halo2-outer-backend-bn254-host",
      Self::MidnightBls12381Host => "midnight-direct-halo2-outer-backend-bls12-host",
    }
  }
}

fn init_tracing() -> Result<()> {
  fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .with_target(false)
    .try_init()
    .map_err(|error| anyhow::anyhow!("failed to initialize tracing subscriber: {error}"))
}

fn configure_direct_execution_runtime() {
  let explicit_thread_count = std::env::var("WRAPPER_DIRECT_RAYON_THREADS")
    .ok()
    .and_then(|value| value.parse::<usize>().ok())
    .filter(|value| *value > 0);

  let builder = match explicit_thread_count {
    Some(thread_count) => ThreadPoolBuilder::new().num_threads(thread_count),
    None => ThreadPoolBuilder::new(),
  };

  if let Ok(()) = builder.build_global() {
    let active_thread_count = rayon::current_num_threads();
    match explicit_thread_count {
      Some(thread_count) => {
        info!(
          "configured direct execution rayon thread pool with {} thread(s) from explicit override (requested {})",
          active_thread_count,
          thread_count
        );
      }
      None => {
        info!(
          "configured direct execution rayon thread pool with {} thread(s) from Rayon default sizing",
          active_thread_count
        );
      }
    }
  } else {
    let active_thread_count = rayon::current_num_threads();
    match explicit_thread_count {
      Some(thread_count) => {
        info!(
          "rayon global thread pool was already initialized before direct execution; using existing {} thread(s) after explicit request for {}",
          active_thread_count,
          thread_count
        );
      }
      None => {
        info!(
          "rayon global thread pool was already initialized before direct execution; using existing {} thread(s)",
          active_thread_count
        );
      }
    }
  }
}

fn direct_execution_log_dir() -> PathBuf {
  std::env::var("HOME")
    .map_or_else(|_| PathBuf::from("."), PathBuf::from)
    .join("tmp")
}

fn direct_execution_log_path(
  command: &str,
  job_id: &str,
  backend_id: &str,
) -> PathBuf {
  let backend_slug = backend_id.replace('/', "-");
  direct_execution_log_dir().join(format!("{command}-{job_id}-{backend_slug}.log"))
}

fn append_direct_execution_log(
  command: &str,
  job_id: &str,
  backend_id: &str,
  message: &str,
) -> Result<()> {
  let dir = direct_execution_log_dir();
  fs::create_dir_all(&dir)
    .with_context(|| format!("failed to create direct execution log dir {}", dir.display()))?;
  let path = direct_execution_log_path(command, job_id, backend_id);
  let timestamp = chrono_like_timestamp();
  let line = format!("{timestamp} {message}\n");
  let mut file = fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(&path)
    .with_context(|| format!("failed to open direct execution log file {}", path.display()))?;
  file
    .write_all(line.as_bytes())
    .with_context(|| format!("failed to append direct execution log file {}", path.display()))?;
  Ok(())
}

fn chrono_like_timestamp() -> String {
  // Lightweight local timestamp for log lines without adding another dependency.
  use std::time::{SystemTime, UNIX_EPOCH};
  let now = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default();
  format!("[{}.{:03}]", now.as_secs(), now.subsec_millis())
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
    let result = unsafe { libc::setrlimit(libc::RLIMIT_AS, &raw const limit) };
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

fn run_search_exp_by_x_chain(args: ExpByXSearchArgs<'_>) {
  let ExpByXSearchArgs {
    max_window,
    max_square_count,
    max_steps,
    limit,
    weight_profile,
    square_cost,
    positive_mul_cost,
    negative_mul_cost,
    unique_window_cost,
    start_window_cost,
    allowed_windows,
  } = args;
  let weights = match weight_profile {
    ExpByXWeightProfile::Linear => Bn254ExpByXChainSearchWeights::linear(
      square_cost,
      positive_mul_cost,
      negative_mul_cost,
      unique_window_cost,
      start_window_cost,
    ),
    ExpByXWeightProfile::Empirical => {
      let mut square_block_costs = [0_u64; 13];
      let capped_square_count = max_square_count.min(12);
      for square_count in 1..=capped_square_count {
        square_block_costs[usize::from(square_count)] =
          fp12_compressed_cyclotomic_square_block_layout_metrics(square_count).rows as u64;
      }
      for index in (usize::from(capped_square_count) + 1)..square_block_costs.len() {
        square_block_costs[index] = square_block_costs[usize::from(capped_square_count)];
      }

      Bn254ExpByXChainSearchWeights {
        square_block_costs,
        positive_mul_cost: fp12_mul_layout_metrics().rows as u64,
        negative_mul_cost: fp12_mul_by_unitary_inverse_layout_metrics().rows as u64,
        unique_window_cost,
        start_window_cost,
      }
    }
  };

  let config = Bn254ExpByXChainSearchConfig {
    max_window,
    max_square_count,
    max_steps,
    max_candidates: limit,
    weights,
  };
  let retained = retained_bn254_exp_by_x_chain_candidate(weights);
  let candidates = if allowed_windows.is_empty() {
    search_bn254_exp_by_x_candidates(config)
  } else {
    search_bn254_exp_by_x_candidates_with_windows(config, allowed_windows)
  };

  println!("retained:");
  println!("  weight_profile={weight_profile:?}");
  println!(
    "  square_block_costs={:?}",
    &weights.square_block_costs[1..=usize::from(max_square_count.min(12))]
  );
  println!(
    "  total_cost={} square_cost={} multiply_cost={} precompute_cost={} start={} windows={:?}",
    retained.total_cost,
    retained.square_cost_total,
    retained.multiply_cost_total,
    retained.precompute_cost_total,
    retained.start_window,
    retained.unique_windows()
  );
  println!("  schedule={}", retained.schedule_string());

  println!("candidates:");
  for (index, candidate) in candidates.iter().take(limit).enumerate() {
    println!(
      "  {}. total_cost={} square_cost={} multiply_cost={} precompute_cost={} start={} windows={:?}",
      index + 1,
      candidate.total_cost,
      candidate.square_cost_total,
      candidate.multiply_cost_total,
      candidate.precompute_cost_total,
      candidate.start_window,
      candidate.unique_windows()
    );
    println!("     schedule={}", candidate.schedule_string());
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

  if matches!(family, ProfileFamily::All | ProfileFamily::FloorPlanner) {
    rows.extend([
      LayoutProfileRow {
        family: "floor_planner",
        id: "simple_bn254_final_exponentiation_hard_part".to_owned(),
        label: "simple planner bn254 final exponentiation hard part",
        term_count: None,
        public_input_count: None,
        ..timed_layout_profile_row(groth16_pairing_block_final_exponentiation_hard_part_layout_metrics)
      },
      LayoutProfileRow {
        family: "floor_planner",
        id: "v1_bn254_final_exponentiation_hard_part".to_owned(),
        label: "v1 planner bn254 final exponentiation hard part",
        term_count: None,
        public_input_count: None,
        ..timed_layout_profile_row(groth16_pairing_block_final_exponentiation_hard_part_layout_metrics_v1)
      },
      LayoutProfileRow {
        family: "floor_planner",
        id: "simple_bn254_pairing_check_groth16_style".to_owned(),
        label: "simple planner bn254 pairing check groth16-style",
        term_count: Some(4),
        public_input_count: Some(1),
        ..timed_layout_profile_row(groth16_pairing_block_pairing_check_groth16_style_layout_metrics)
      },
      LayoutProfileRow {
        family: "floor_planner",
        id: "v1_bn254_pairing_check_groth16_style".to_owned(),
        label: "v1 planner bn254 pairing check groth16-style",
        term_count: Some(4),
        public_input_count: Some(1),
        ..timed_layout_profile_row(groth16_pairing_block_pairing_check_groth16_style_layout_metrics_v1)
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
      |fixture: OuterProfileFixtureArg| outer_fixture.is_none_or(|selected| selected == fixture);
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
  let setup_result: DirectWrapperSetupExecutionResultJson = serde_json::from_slice(&setup_bundle_json)
    .with_context(|| format!("failed to parse setup bundle at {}", setup_path.display()))?;
  let setup_bundle = setup_result.setup_bundle;
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

fn run_execute_wrapper_direct_prove_trace(
  identifier: &str,
  proof_path: &PathBuf,
  public_path: &PathBuf,
  vk_path: &PathBuf,
  public_input_names: &[String],
  backend_arg: DirectOuterBackendArg,
  setup_path: &Path,
  output_path: &Path,
) -> Result<()> {
  info!("running direct wrapper prove trace {}", identifier);
  let log_path = direct_execution_log_path(
    "execute-wrapper-direct-prove-trace",
    identifier,
    backend_arg.backend_id_hint(),
  );
  // Safety: this CLI is single-process command execution code and we set the
  // process environment before entering the heavy backend work so the backend
  // can discover one log-file path for this invocation.
  unsafe { std::env::set_var("WRAPPER_DIRECT_LOG_FILE", &log_path) };
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
  let setup_result: DirectWrapperSetupExecutionResultJson = serde_json::from_slice(&setup_bundle_json)
    .with_context(|| format!("failed to parse setup bundle at {}", setup_path.display()))?;
  let setup_bundle = setup_result.setup_bundle;
  let artifacts = OuterCircuitInputArtifacts::new(
    Some(proof_json.as_slice()),
    Some(verification_key_json.as_slice()),
  );
  let result = match backend_arg {
    DirectOuterBackendArg::MidnightBn254Host => {
      execute_wrapper_direct_prove_trace_with_bn254_backend(
        &MidnightDirectOuterBackendBn254Host,
        &package,
        artifacts,
        setup_path,
        &setup_bundle,
        output_path,
      )?
    }
    DirectOuterBackendArg::MidnightBls12381Host => {
      execute_wrapper_direct_prove_trace_with_bls12_backend(
        &MidnightDirectOuterBackendBls12Host,
        &package,
        artifacts,
        setup_path,
        &setup_bundle,
        output_path,
      )?
    }
  };
  emit_json(&result, None, "direct wrapper prove trace result")
}

fn run_execute_wrapper_direct_prove_finalize(args: DirectProveFinalizeArgs<'_>) -> Result<()> {
  let DirectProveFinalizeArgs {
    identifier,
    proof_path,
    public_path,
    vk_path,
    public_input_names,
    backend_arg,
    setup_path,
    trace_path,
    h_poly_row_chunk_size,
    output_path,
  } = args;
  info!("running direct wrapper prove finalize {}", identifier);
  let log_path = direct_execution_log_path(
    "execute-wrapper-direct-prove-finalize",
    identifier,
    backend_arg.backend_id_hint(),
  );
  // Safety: this CLI is single-process command execution code and we set the
  // process environment before entering the heavy backend work so the backend
  // can discover one log-file path for this invocation.
  unsafe { std::env::set_var("WRAPPER_DIRECT_LOG_FILE", &log_path) };
  if let Some(row_chunk_log2) = h_poly_row_chunk_size {
    let row_chunk_size = 1_usize
      .checked_shl(row_chunk_log2)
      .context("h_poly row chunk size exponent is too large for this machine word size")?;
    // Safety: same rationale as the direct log-file env var above; this is
    // process-local configuration applied before backend work begins.
    unsafe {
      std::env::set_var("WRAPPER_H_POLY_ROW_CHUNK_SIZE", row_chunk_size.to_string());
    }
    info!(
      "configured prove-finalize h_poly row chunk size override to 2^{} = {} row(s)",
      row_chunk_log2,
      row_chunk_size
    );
  }
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
  let setup_result: DirectWrapperSetupExecutionResultJson = serde_json::from_slice(&setup_bundle_json)
    .with_context(|| format!("failed to parse setup bundle at {}", setup_path.display()))?;
  let setup_bundle = setup_result.setup_bundle;
  let artifacts = OuterCircuitInputArtifacts::new(
    Some(proof_json.as_slice()),
    Some(verification_key_json.as_slice()),
  );
  let result = match backend_arg {
    DirectOuterBackendArg::MidnightBn254Host => {
      execute_wrapper_direct_prove_finalize_with_bn254_backend(
        &MidnightDirectOuterBackendBn254Host,
        &package,
        artifacts,
        setup_path,
        &setup_bundle,
        trace_path,
      )?
    }
    DirectOuterBackendArg::MidnightBls12381Host => {
      execute_wrapper_direct_prove_finalize_with_bls12_backend(
        &MidnightDirectOuterBackendBls12Host,
        &package,
        artifacts,
        setup_path,
        &setup_bundle,
        trace_path,
      )?
    }
  };
  emit_json(&result, Some(output_path), "direct wrapper produced proof bundle")
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
  output_path: &Path,
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
      "setup result persists verification materials plus a proving-key sidecar".to_owned(),
    ],
  })
}

fn execute_wrapper_direct_setup_with_bls12_backend(
  backend: &MidnightDirectOuterBackendBls12Host,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  output_path: &Path,
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
      "setup result persists verification materials plus a proving-key sidecar".to_owned(),
    ],
  })
}

fn execute_wrapper_direct_prove_with_bn254_backend(
  backend: &MidnightDirectOuterBackendBn254Host,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  setup_path: &Path,
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
    .context("direct wrapper prove failed while reusing persisted proving-key sidecar")?;

  Ok(DirectWrapperProveExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    prove_elapsed_ms: started_at.elapsed().as_millis(),
    produced_bundle,
    notes: vec![
      format!("executed direct outer prove path with backend {}", backend.backend_id()),
      "prove reused a persisted proving-key sidecar and avoided rerunning keygen_pk".to_owned(),
    ],
  })
}

fn execute_wrapper_direct_prove_with_bls12_backend(
  backend: &MidnightDirectOuterBackendBls12Host,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  setup_path: &Path,
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
    .context("direct wrapper prove failed while reusing persisted BLS12 proving-key sidecar")?;

  Ok(DirectWrapperProveExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    prove_elapsed_ms: started_at.elapsed().as_millis(),
    produced_bundle,
    notes: vec![
      format!("executed direct outer prove path with backend {}", backend.backend_id()),
      "prove reused a persisted proving-key sidecar and avoided rerunning keygen_pk".to_owned(),
    ],
  })
}

fn proving_key_sidecar_path(setup_manifest_path: &Path) -> PathBuf {
  let mut file_name = setup_manifest_path
    .file_name()
    .and_then(|name| name.to_str())
    .unwrap_or("direct-setup.json")
    .to_owned();
  file_name.push_str(".pk");
  setup_manifest_path.with_file_name(file_name)
}

fn resolve_setup_sidecar_path(
  setup_manifest_path: &Path,
  setup_bundle: &ProducedOuterSetupArtifactBundle,
) -> PathBuf {
  setup_manifest_path.with_file_name(&setup_bundle.proving_key.proving_key_artifact)
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

fn execute_wrapper_direct_prove_trace_with_bn254_backend(
  backend: &MidnightDirectOuterBackendBn254Host,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  setup_path: &Path,
  setup_bundle: &ProducedOuterSetupArtifactBundle,
  output_path: &Path,
) -> Result<DirectWrapperProveTraceExecutionResult> {
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-trace",
    &package.job.identifier,
    backend.backend_id(),
    "start",
  );
  let started_at = Instant::now();
  let circuit = backend
    .build_outer_circuit(package, artifacts)
    .context("direct wrapper prove trace failed while building outer circuit")?;
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-trace",
    &package.job.identifier,
    backend.backend_id(),
    "outer circuit built",
  );
  let proving_key_path = resolve_setup_sidecar_path(setup_path, setup_bundle);
  let proving_key_file = fs::File::open(&proving_key_path).with_context(|| {
    format!("failed to open proving-key sidecar file at {}", proving_key_path.display())
  })?;
  let mut proving_key_reader = BufReader::new(proving_key_file);
  let trace_file = fs::File::create(output_path)
    .with_context(|| format!("failed to create prover trace artifact at {}", output_path.display()))?;
  let mut trace_writer = BufWriter::new(trace_file);
  backend
    .produce_proof_trace_from_setup_reader(
      package,
      &circuit,
      setup_bundle,
      &mut proving_key_reader,
      &mut trace_writer,
    )
    .context("direct wrapper prove trace failed while producing persisted prover trace")?;
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-trace",
    &package.job.identifier,
    backend.backend_id(),
    &format!(
      "prove trace complete ({} ms) -> {}",
      started_at.elapsed().as_millis(),
      output_path.display()
    ),
  );

  Ok(DirectWrapperProveTraceExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    trace_elapsed_ms: started_at.elapsed().as_millis(),
    trace_artifact: output_path.display().to_string(),
    notes: vec![
      format!("executed direct outer prove first stage with backend {}", backend.backend_id()),
      "trace artifact captures proving state immediately before the compute_h_poly/finalization stage"
        .to_owned(),
    ],
  })
}

fn execute_wrapper_direct_prove_trace_with_bls12_backend(
  backend: &MidnightDirectOuterBackendBls12Host,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  setup_path: &Path,
  setup_bundle: &ProducedOuterSetupArtifactBundle,
  output_path: &Path,
) -> Result<DirectWrapperProveTraceExecutionResult> {
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-trace",
    &package.job.identifier,
    backend.backend_id(),
    "start",
  );
  let started_at = Instant::now();
  let circuit = backend
    .build_outer_circuit(package, artifacts)
    .context("direct wrapper BLS12 prove trace failed while building outer circuit")?;
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-trace",
    &package.job.identifier,
    backend.backend_id(),
    "outer circuit built",
  );
  let proving_key_path = resolve_setup_sidecar_path(setup_path, setup_bundle);
  let proving_key_file = fs::File::open(&proving_key_path).with_context(|| {
    format!("failed to open proving-key sidecar file at {}", proving_key_path.display())
  })?;
  let mut proving_key_reader = BufReader::new(proving_key_file);
  let trace_file = fs::File::create(output_path)
    .with_context(|| format!("failed to create prover trace artifact at {}", output_path.display()))?;
  let mut trace_writer = BufWriter::new(trace_file);
  backend
    .produce_proof_trace_from_setup_reader(
      package,
      &circuit,
      setup_bundle,
      &mut proving_key_reader,
      &mut trace_writer,
    )
    .context("direct wrapper BLS12 prove trace failed while producing persisted prover trace")?;
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-trace",
    &package.job.identifier,
    backend.backend_id(),
    &format!(
      "prove trace complete ({} ms) -> {}",
      started_at.elapsed().as_millis(),
      output_path.display()
    ),
  );

  Ok(DirectWrapperProveTraceExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    trace_elapsed_ms: started_at.elapsed().as_millis(),
    trace_artifact: output_path.display().to_string(),
    notes: vec![
      format!("executed direct outer prove first stage with backend {}", backend.backend_id()),
      "trace artifact captures proving state immediately before the compute_h_poly/finalization stage"
        .to_owned(),
    ],
  })
}

fn execute_wrapper_direct_prove_finalize_with_bn254_backend(
  backend: &MidnightDirectOuterBackendBn254Host,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  setup_path: &Path,
  setup_bundle: &ProducedOuterSetupArtifactBundle,
  trace_path: &Path,
) -> Result<DirectWrapperProveExecutionResult> {
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-finalize",
    &package.job.identifier,
    backend.backend_id(),
    "start",
  );
  let started_at = Instant::now();
  let circuit = backend
    .build_outer_circuit(package, artifacts)
    .context("direct wrapper prove finalize failed while building outer circuit")?;
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-finalize",
    &package.job.identifier,
    backend.backend_id(),
    "outer circuit built",
  );
  let proving_key_path = resolve_setup_sidecar_path(setup_path, setup_bundle);
  let proving_key_file = fs::File::open(&proving_key_path).with_context(|| {
    format!("failed to open proving-key sidecar file at {}", proving_key_path.display())
  })?;
  let mut proving_key_reader = BufReader::new(proving_key_file);
  let trace_file = fs::File::open(trace_path)
    .with_context(|| format!("failed to open prover trace artifact at {}", trace_path.display()))?;
  let mut trace_reader = BufReader::new(trace_file);
  let produced_bundle = backend
    .produce_proof_bundle_from_trace_reader(
      package,
      &circuit,
      setup_bundle,
      &mut proving_key_reader,
      &mut trace_reader,
    )
    .context("direct wrapper prove finalize failed while finalizing from persisted prover trace")?;
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-finalize",
    &package.job.identifier,
    backend.backend_id(),
    &format!("prove finalize complete ({} ms)", started_at.elapsed().as_millis()),
  );

  Ok(DirectWrapperProveExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    prove_elapsed_ms: started_at.elapsed().as_millis(),
    produced_bundle,
    notes: vec![
      format!("executed direct outer prove finalization path with backend {}", backend.backend_id()),
      "prove finalization resumed from a persisted pre-compute_h_poly trace artifact".to_owned(),
    ],
  })
}

fn execute_wrapper_direct_prove_finalize_with_bls12_backend(
  backend: &MidnightDirectOuterBackendBls12Host,
  package: &WrapperExecutionPackage,
  artifacts: OuterCircuitInputArtifacts<'_>,
  setup_path: &Path,
  setup_bundle: &ProducedOuterSetupArtifactBundle,
  trace_path: &Path,
) -> Result<DirectWrapperProveExecutionResult> {
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-finalize",
    &package.job.identifier,
    backend.backend_id(),
    "start",
  );
  let started_at = Instant::now();
  let circuit = backend
    .build_outer_circuit(package, artifacts)
    .context("direct wrapper BLS12 prove finalize failed while building outer circuit")?;
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-finalize",
    &package.job.identifier,
    backend.backend_id(),
    "outer circuit built",
  );
  let proving_key_path = resolve_setup_sidecar_path(setup_path, setup_bundle);
  let proving_key_file = fs::File::open(&proving_key_path).with_context(|| {
    format!("failed to open proving-key sidecar file at {}", proving_key_path.display())
  })?;
  let mut proving_key_reader = BufReader::new(proving_key_file);
  let trace_file = fs::File::open(trace_path)
    .with_context(|| format!("failed to open prover trace artifact at {}", trace_path.display()))?;
  let mut trace_reader = BufReader::new(trace_file);
  let produced_bundle = backend
    .produce_proof_bundle_from_trace_reader(
      package,
      &circuit,
      setup_bundle,
      &mut proving_key_reader,
      &mut trace_reader,
    )
    .context("direct wrapper BLS12 prove finalize failed while finalizing from persisted prover trace")?;
  let _ = append_direct_execution_log(
    "execute-wrapper-direct-prove-finalize",
    &package.job.identifier,
    backend.backend_id(),
    &format!("prove finalize complete ({} ms)", started_at.elapsed().as_millis()),
  );

  Ok(DirectWrapperProveExecutionResult {
    job_id: package.job.identifier.clone(),
    backend: backend.backend_id().to_owned(),
    outer_host: backend.metadata().outer_host.id().to_owned(),
    prove_elapsed_ms: started_at.elapsed().as_millis(),
    produced_bundle,
    notes: vec![
      format!("executed direct outer prove finalization path with backend {}", backend.backend_id()),
      "prove finalization resumed from a persisted pre-compute_h_poly trace artifact".to_owned(),
    ],
  })
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
