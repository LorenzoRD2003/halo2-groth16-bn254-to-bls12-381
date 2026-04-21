//! ECC benchmark hooks backed by the real Midnight BN254 G1 circuit.

use criterion::{BenchmarkId, Criterion};
use midnight_proofs::dev::MockProver;
use wrapper_circuits::{G1AddCircuit, g1_add_k};

fn run_g1_add_circuit() {
  let circuit = G1AddCircuit::sample();
  let prover = MockProver::run(g1_add_k(), &circuit, vec![vec![], vec![]])
    .expect("g1 add circuit should build");

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
