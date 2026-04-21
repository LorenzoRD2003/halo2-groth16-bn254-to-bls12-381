//! Field benchmark hooks backed by the real Midnight BN254 foreign-field circuits.

use criterion::{BenchmarkId, Criterion};
use midnight_proofs::dev::MockProver;
use wrapper_circuits::{FpAddCircuit, FpMulCircuit, fp_add_k, fp_mul_k};

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
