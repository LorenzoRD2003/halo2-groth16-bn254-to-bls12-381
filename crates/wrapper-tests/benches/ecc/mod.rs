//! ECC benchmark hooks backed by the real Midnight BN254 G1 circuit.

use criterion::{BenchmarkId, Criterion};
use midnight_proofs::dev::MockProver;
use wrapper_circuits::{
  G1AddCircuit, G2NegCircuit, G2OnCurveCircuit, g1_add_k, g2_neg_k, g2_on_curve_k,
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
