//! ECC benchmark hooks backed by the real Midnight BN254 G1 circuit.

use criterion::{BenchmarkId, Criterion};
use midnight_proofs::dev::MockProver;
use wrapper_circuits::{
  G1AddCircuit, G2NegCircuit, G2OnCurveCircuit, G2ProjectiveAddCircuit, G2ProjectiveDoubleCircuit,
  G2ProjectiveFromAffineCircuit, g1_add_k, g2_neg_k, g2_on_curve_k, g2_proj_add_k,
  g2_proj_double_k, g2_proj_from_affine_k,
};

fn run_g1_add_circuit() {
  let circuit = G1AddCircuit::sample();
  let prover = MockProver::run(g1_add_k(), &circuit, vec![vec![], vec![]])
    .expect("g1 add circuit should build");

  assert_eq!(prover.verify(), Ok(()));
}

fn run_g2_on_curve_circuit() {
  let circuit = G2OnCurveCircuit::sample();
  let prover = MockProver::run(g2_on_curve_k(), &circuit, vec![vec![], vec![]])
    .expect("g2 on-curve circuit should build");

  assert_eq!(prover.verify(), Ok(()));
}

fn run_g2_neg_circuit() {
  let circuit = G2NegCircuit::sample();
  let prover = MockProver::run(g2_neg_k(), &circuit, vec![vec![], vec![]])
    .expect("g2 neg circuit should build");

  assert_eq!(prover.verify(), Ok(()));
}

fn run_g2_proj_from_affine_circuit() {
  let circuit = G2ProjectiveFromAffineCircuit::sample();
  let prover = MockProver::run(g2_proj_from_affine_k(), &circuit, vec![vec![], vec![]])
    .expect("g2 projective from_affine circuit should build");

  assert_eq!(prover.verify(), Ok(()));
}

fn run_g2_proj_double_circuit() {
  let circuit = G2ProjectiveDoubleCircuit::sample();
  let prover = MockProver::run(g2_proj_double_k(), &circuit, vec![vec![], vec![]])
    .expect("g2 projective double circuit should build");

  assert_eq!(prover.verify(), Ok(()));
}

fn run_g2_proj_add_circuit() {
  let circuit = G2ProjectiveAddCircuit::sample();
  let prover = MockProver::run(g2_proj_add_k(), &circuit, vec![vec![], vec![]])
    .expect("g2 projective add circuit should build");

  assert_eq!(prover.verify(), Ok(()));
}

/// Benchmarks the current Midnight-backed BN254 G1 addition circuit.
pub fn bench_g1_add(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("ecc");
  group.bench_with_input(BenchmarkId::new("bench_g1_add", 1), &1_u8, |bench, _| {
    bench.iter(run_g1_add_circuit);
  });
  group.finish();
}

/// Benchmarks the current Midnight-backed BN254 G2 on-curve circuit.
pub fn bench_g2_on_curve(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("ecc");
  group.bench_with_input(BenchmarkId::new("bench_g2_on_curve", 1), &1_u8, |bench, _| {
    bench.iter(run_g2_on_curve_circuit);
  });
  group.finish();
}

/// Benchmarks the current Midnight-backed BN254 G2 negation circuit.
pub fn bench_g2_neg(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("ecc");
  group.bench_with_input(BenchmarkId::new("bench_g2_neg", 1), &1_u8, |bench, _| {
    bench.iter(run_g2_neg_circuit);
  });
  group.finish();
}

/// Benchmarks the current Midnight-backed BN254 G2 affine-to-projective embedding circuit.
pub fn bench_g2_proj_from_affine(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("ecc");
  group.bench_with_input(BenchmarkId::new("bench_g2_proj_from_affine", 1), &1_u8, |bench, _| {
    bench.iter(run_g2_proj_from_affine_circuit);
  });
  group.finish();
}

/// Benchmarks the current Midnight-backed BN254 G2 projective doubling circuit.
pub fn bench_g2_proj_double(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("ecc");
  group.bench_with_input(BenchmarkId::new("bench_g2_proj_double", 1), &1_u8, |bench, _| {
    bench.iter(run_g2_proj_double_circuit);
  });
  group.finish();
}

/// Benchmarks the current Midnight-backed BN254 G2 projective addition circuit.
pub fn bench_g2_proj_add(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("ecc");
  group.bench_with_input(BenchmarkId::new("bench_g2_proj_add", 1), &1_u8, |bench, _| {
    bench.iter(run_g2_proj_add_circuit);
  });
  group.finish();
}
