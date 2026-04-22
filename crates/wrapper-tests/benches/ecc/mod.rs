//! ECC benchmark hooks backed by the real Midnight BN254 G1 circuit.

use criterion::{BenchmarkId, Criterion};
use midnight_proofs::dev::MockProver;
use midnight_proofs::plonk::Circuit;
use wrapper_circuits::{
  G1AddCircuit, G2DoubleWithLineCircuit, G2MixedAddWithLineCircuit, G2NegCircuit, G2OnCurveCircuit,
  G2ProjectiveAddCircuit, G2ProjectiveDoubleCircuit, G2ProjectiveFromAffineCircuit,
  MillerAccumulatorMulByLineCircuit, MillerAccumulatorSquareCircuit, MillerLoopCircuit,
  NativeField, g1_add_k, g2_double_with_line_k, g2_mixed_add_with_line_k, g2_neg_k, g2_on_curve_k,
  g2_proj_add_k, g2_proj_double_k, g2_proj_from_affine_k, miller_accumulator_mul_by_line_k,
  miller_accumulator_square_k, miller_loop_k,
};

fn verify_sample_circuit<CircuitT>(circuit: &CircuitT, k: u32, build_error: &str)
where
  CircuitT: Circuit<NativeField>,
{
  let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect(build_error);
  assert_eq!(prover.verify(), Ok(()));
}

fn bench_verified_sample(criterion: &mut Criterion, bench_name: &str, run: fn()) {
  let mut group = criterion.benchmark_group("ecc");
  group.bench_with_input(BenchmarkId::new(bench_name, 1), &1_u8, |bench, _| bench.iter(run));
  group.finish();
}

fn run_g1_add_circuit() {
  verify_sample_circuit(&G1AddCircuit::sample(), g1_add_k(), "g1 add circuit should build");
}

fn run_g2_on_curve_circuit() {
  verify_sample_circuit(
    &G2OnCurveCircuit::sample(),
    g2_on_curve_k(),
    "g2 on-curve circuit should build",
  );
}

fn run_g2_neg_circuit() {
  verify_sample_circuit(&G2NegCircuit::sample(), g2_neg_k(), "g2 neg circuit should build");
}

fn run_g2_proj_from_affine_circuit() {
  verify_sample_circuit(
    &G2ProjectiveFromAffineCircuit::sample(),
    g2_proj_from_affine_k(),
    "g2 projective from_affine circuit should build",
  );
}

fn run_g2_proj_double_circuit() {
  verify_sample_circuit(
    &G2ProjectiveDoubleCircuit::sample(),
    g2_proj_double_k(),
    "g2 projective double circuit should build",
  );
}

fn run_g2_proj_add_circuit() {
  verify_sample_circuit(
    &G2ProjectiveAddCircuit::sample(),
    g2_proj_add_k(),
    "g2 projective add circuit should build",
  );
}

fn run_g2_double_with_line_circuit() {
  verify_sample_circuit(
    &G2DoubleWithLineCircuit::sample(),
    g2_double_with_line_k(),
    "g2 double_with_line circuit should build",
  );
}

fn run_g2_mixed_add_with_line_circuit() {
  verify_sample_circuit(
    &G2MixedAddWithLineCircuit::sample(),
    g2_mixed_add_with_line_k(),
    "g2 mixed_add_with_line circuit should build",
  );
}

fn run_miller_accumulator_square_circuit() {
  verify_sample_circuit(
    &MillerAccumulatorSquareCircuit::sample(),
    miller_accumulator_square_k(),
    "miller accumulator square circuit should build",
  );
}

fn run_miller_accumulator_mul_by_line_circuit() {
  verify_sample_circuit(
    &MillerAccumulatorMulByLineCircuit::sample(),
    miller_accumulator_mul_by_line_k(),
    "miller accumulator mul_by_line circuit should build",
  );
}

fn run_miller_loop_circuit() {
  verify_sample_circuit(
    &MillerLoopCircuit::sample(),
    miller_loop_k(),
    "miller loop circuit should build",
  );
}

/// Benchmarks the current Midnight-backed BN254 G1 addition circuit.
pub fn bench_g1_add(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "bench_g1_add", run_g1_add_circuit);
}

/// Benchmarks the current Midnight-backed BN254 G2 on-curve circuit.
pub fn bench_g2_on_curve(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "bench_g2_on_curve", run_g2_on_curve_circuit);
}

/// Benchmarks the current Midnight-backed BN254 G2 negation circuit.
pub fn bench_g2_neg(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "bench_g2_neg", run_g2_neg_circuit);
}

/// Benchmarks the current Midnight-backed BN254 G2 affine-to-projective embedding circuit.
pub fn bench_g2_proj_from_affine(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "bench_g2_proj_from_affine", run_g2_proj_from_affine_circuit);
}

/// Benchmarks the current Midnight-backed BN254 G2 projective doubling circuit.
pub fn bench_g2_proj_double(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "bench_g2_proj_double", run_g2_proj_double_circuit);
}

/// Benchmarks the current Midnight-backed BN254 G2 projective addition circuit.
pub fn bench_g2_proj_add(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "bench_g2_proj_add", run_g2_proj_add_circuit);
}

/// Benchmarks the current Midnight-backed BN254 G2 doubling-with-line circuit.
pub fn bench_g2_double_with_line(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "bench_g2_double_with_line", run_g2_double_with_line_circuit);
}

/// Benchmarks the current Midnight-backed BN254 G2 mixed-add-with-line circuit.
pub fn bench_g2_mixed_add_with_line(criterion: &mut Criterion) {
  bench_verified_sample(
    criterion,
    "bench_g2_mixed_add_with_line",
    run_g2_mixed_add_with_line_circuit,
  );
}

/// Benchmarks the current Midnight-backed BN254 Miller-accumulator square circuit.
pub fn bench_miller_accumulator_square(criterion: &mut Criterion) {
  bench_verified_sample(
    criterion,
    "bench_miller_accumulator_square",
    run_miller_accumulator_square_circuit,
  );
}

/// Benchmarks the current Midnight-backed BN254 Miller-accumulator mul-by-line circuit.
pub fn bench_miller_accumulator_mul_by_line(criterion: &mut Criterion) {
  bench_verified_sample(
    criterion,
    "bench_miller_accumulator_mul_by_line",
    run_miller_accumulator_mul_by_line_circuit,
  );
}

/// Benchmarks the current narrow Midnight-backed BN254 Miller-loop circuit.
pub fn bench_miller_loop_narrow(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "bench_miller_loop_narrow", run_miller_loop_circuit);
}
