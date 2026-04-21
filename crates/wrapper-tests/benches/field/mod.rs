//! Field benchmark hooks backed by the real Midnight BN254 foreign-field circuits.

use criterion::{BenchmarkId, Criterion};
use midnight_proofs::dev::MockProver;
use wrapper_circuits::{
  Fp2AddCircuit, Fp2MulCircuit, Fp2SquareCircuit, FpAddCircuit, FpMulCircuit, fp_add_k, fp_mul_k,
  fp2_add_k, fp2_mul_k, fp2_square_k,
};

fn run_fp_add_circuit() {
  let circuit = FpAddCircuit::sample();
  let prover = MockProver::run(fp_add_k(), &circuit, vec![vec![], vec![]])
    .expect("field add circuit should build");

  assert_eq!(prover.verify(), Ok(()));
}

fn run_fp_mul_circuit() {
  let circuit = FpMulCircuit::sample();
  let prover = MockProver::run(fp_mul_k(), &circuit, vec![vec![], vec![]])
    .expect("field mul circuit should build");

  assert_eq!(prover.verify(), Ok(()));
}

fn run_fp2_add_circuit() {
  let circuit = Fp2AddCircuit::sample();
  let prover = MockProver::run(fp2_add_k(), &circuit, vec![vec![], vec![]])
    .expect("fp2 add circuit should build");

  assert_eq!(prover.verify(), Ok(()));
}

fn run_fp2_mul_circuit() {
  let circuit = Fp2MulCircuit::sample();
  let prover = MockProver::run(fp2_mul_k(), &circuit, vec![vec![], vec![]])
    .expect("fp2 mul circuit should build");

  assert_eq!(prover.verify(), Ok(()));
}

fn run_fp2_square_circuit() {
  let circuit = Fp2SquareCircuit::sample();
  let prover = MockProver::run(fp2_square_k(), &circuit, vec![vec![], vec![]])
    .expect("fp2 square circuit should build");

  assert_eq!(prover.verify(), Ok(()));
}

/// Benchmarks the current Midnight-backed BN254 foreign-field addition circuit.
pub fn bench_fp_add(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("field");
  group.bench_with_input(BenchmarkId::new("bench_fp_add", 1), &1_u8, |bench, _| {
    bench.iter(run_fp_add_circuit);
  });
  group.finish();
}

/// Benchmarks the current Midnight-backed BN254 foreign-field multiplication circuit.
pub fn bench_fp_mul(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("field");
  group.bench_with_input(BenchmarkId::new("bench_fp_mul", 1), &1_u8, |bench, _| {
    bench.iter(run_fp_mul_circuit);
  });
  group.finish();
}

/// Benchmarks the current Midnight-backed BN254 Fp2 addition circuit.
pub fn bench_fp2_add(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("field");
  group.bench_with_input(BenchmarkId::new("bench_fp2_add", 1), &1_u8, |bench, _| {
    bench.iter(run_fp2_add_circuit);
  });
  group.finish();
}

/// Benchmarks the current Midnight-backed BN254 Fp2 multiplication circuit.
pub fn bench_fp2_mul(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("field");
  group.bench_with_input(BenchmarkId::new("bench_fp2_mul", 1), &1_u8, |bench, _| {
    bench.iter(run_fp2_mul_circuit);
  });
  group.finish();
}

/// Benchmarks the current Midnight-backed BN254 Fp2 square circuit.
pub fn bench_fp2_square(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("field");
  group.bench_with_input(BenchmarkId::new("bench_fp2_square", 1), &1_u8, |bench, _| {
    bench.iter(run_fp2_square_circuit);
  });
  group.finish();
}
