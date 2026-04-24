//! Outer-wrapper benchmark hooks over the committed Groth16 fixtures and host lanes.

use criterion::Criterion;
use midnight_proofs::dev::MockProver;
use wrapper_circuits::Bls12HostField;
use wrapper_tests::{
  OuterBenchFixture, build_outer_bench_circuit_bls12, build_outer_bench_circuit_bn254,
};

use crate::common::bench_verified_sample;

fn verify_bn254_outer_fixture(fixture: OuterBenchFixture) {
  let circuit = build_outer_bench_circuit_bn254(fixture);
  let instances = circuit.semantic().input.outer_statement.public_inputs.clone();
  let prover = MockProver::run(22, &circuit, vec![instances, vec![]])
    .expect("BN254 outer circuit should build");
  assert_eq!(prover.verify(), Ok(()));
}

fn verify_bls12_outer_fixture(fixture: OuterBenchFixture) {
  let circuit = build_outer_bench_circuit_bls12(fixture);
  let instances = wrapper_circuits::lift_outer_inputs_to_host::<Bls12HostField>(
    &circuit.semantic().input.outer_statement.public_inputs,
  );
  let prover = MockProver::run(22, &circuit, vec![instances, Vec::new()])
    .expect("BLS12 outer circuit should build");
  assert_eq!(prover.verify(), Ok(()));
}

fn run_outer_circom_multiplier2_bn254_host() {
  verify_bn254_outer_fixture(OuterBenchFixture::CircomMultiplier2);
}

fn run_outer_circom_multiplier2_bls12_381_host() {
  verify_bls12_outer_fixture(OuterBenchFixture::CircomMultiplier2);
}

fn run_outer_semaphore_bn254_host() {
  verify_bn254_outer_fixture(OuterBenchFixture::SemaphoreDepth10);
}

fn run_outer_semaphore_bls12_381_host() {
  verify_bls12_outer_fixture(OuterBenchFixture::SemaphoreDepth10);
}

/// Benchmarks the BN254-hosted direct outer lane on the committed `circom_multiplier2` fixture.
pub fn bench_outer_circom_multiplier2_bn254_host(criterion: &mut Criterion) {
  bench_verified_sample(
    criterion,
    "outer",
    "bench_outer_circom_multiplier2_bn254_host",
    run_outer_circom_multiplier2_bn254_host,
  );
}

/// Benchmarks the BLS12-381-hosted direct outer lane on the committed `circom_multiplier2` fixture.
pub fn bench_outer_circom_multiplier2_bls12_381_host(criterion: &mut Criterion) {
  bench_verified_sample(
    criterion,
    "outer",
    "bench_outer_circom_multiplier2_bls12_381_host",
    run_outer_circom_multiplier2_bls12_381_host,
  );
}

/// Benchmarks the BN254-hosted direct outer lane on the committed Semaphore fixture.
pub fn bench_outer_semaphore_bn254_host(criterion: &mut Criterion) {
  bench_verified_sample(
    criterion,
    "outer",
    "bench_outer_semaphore_bn254_host",
    run_outer_semaphore_bn254_host,
  );
}

/// Benchmarks the BLS12-381-hosted direct outer lane on the committed Semaphore fixture.
pub fn bench_outer_semaphore_bls12_381_host(criterion: &mut Criterion) {
  bench_verified_sample(
    criterion,
    "outer",
    "bench_outer_semaphore_bls12_381_host",
    run_outer_semaphore_bls12_381_host,
  );
}
