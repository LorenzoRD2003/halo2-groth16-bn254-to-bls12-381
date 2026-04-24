//! Field benchmark hooks backed by the real Midnight BN254 foreign-field circuits.

use criterion::Criterion;
use midnight_proofs::dev::MockProver;
use midnight_proofs::plonk::Circuit;
use wrapper_circuits::{
  Fp2AddCircuit, Fp2MulCircuit, Fp2SquareCircuit, Fp6AddCircuit, Fp6MulCircuit, Fp6SquareCircuit,
  Fp12AddCircuit, Fp12CyclotomicSquareCircuit, Fp12MulCircuit, Fp12SquareCircuit, FpAddCircuit,
  FpMulCircuit, NativeField, fp_add_k, fp_mul_k, fp2_add_k, fp2_mul_k, fp2_square_k, fp6_add_k,
  fp6_mul_k, fp6_square_k, fp12_add_k, fp12_cyclotomic_square_k, fp12_mul_k, fp12_square_k,
};

use crate::common::bench_verified_sample;

fn verify_sample_circuit<CircuitT>(circuit: &CircuitT, k: u32, build_error: &str)
where
  CircuitT: Circuit<NativeField>,
{
  let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect(build_error);
  assert_eq!(prover.verify(), Ok(()));
}

fn run_fp_add_circuit() {
  verify_sample_circuit(&FpAddCircuit::sample(), fp_add_k(), "field add circuit should build");
}

fn run_fp_mul_circuit() {
  verify_sample_circuit(&FpMulCircuit::sample(), fp_mul_k(), "field mul circuit should build");
}

fn run_fp2_add_circuit() {
  verify_sample_circuit(&Fp2AddCircuit::sample(), fp2_add_k(), "fp2 add circuit should build");
}

fn run_fp2_mul_circuit() {
  verify_sample_circuit(&Fp2MulCircuit::sample(), fp2_mul_k(), "fp2 mul circuit should build");
}

fn run_fp2_square_circuit() {
  verify_sample_circuit(
    &Fp2SquareCircuit::sample(),
    fp2_square_k(),
    "fp2 square circuit should build",
  );
}

fn run_fp6_add_circuit() {
  verify_sample_circuit(&Fp6AddCircuit::sample(), fp6_add_k(), "fp6 add circuit should build");
}

fn run_fp6_mul_circuit() {
  verify_sample_circuit(&Fp6MulCircuit::sample(), fp6_mul_k(), "fp6 mul circuit should build");
}

fn run_fp6_square_circuit() {
  verify_sample_circuit(
    &Fp6SquareCircuit::sample(),
    fp6_square_k(),
    "fp6 square circuit should build",
  );
}

fn run_fp12_add_circuit() {
  verify_sample_circuit(&Fp12AddCircuit::sample(), fp12_add_k(), "fp12 add circuit should build");
}

fn run_fp12_mul_circuit() {
  verify_sample_circuit(&Fp12MulCircuit::sample(), fp12_mul_k(), "fp12 mul circuit should build");
}

fn run_fp12_square_circuit() {
  verify_sample_circuit(
    &Fp12SquareCircuit::sample(),
    fp12_square_k(),
    "fp12 square circuit should build",
  );
}

fn run_fp12_cyclotomic_square_circuit() {
  verify_sample_circuit(
    &Fp12CyclotomicSquareCircuit::sample(),
    fp12_cyclotomic_square_k(),
    "fp12 cyclotomic square circuit should build",
  );
}

/// Benchmarks the current Midnight-backed BN254 foreign-field addition circuit.
pub fn bench_fp_add(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "field", "bench_fp_add", run_fp_add_circuit);
}

/// Benchmarks the current Midnight-backed BN254 foreign-field multiplication circuit.
pub fn bench_fp_mul(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "field", "bench_fp_mul", run_fp_mul_circuit);
}

/// Benchmarks the current Midnight-backed BN254 Fp2 addition circuit.
pub fn bench_fp2_add(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "field", "bench_fp2_add", run_fp2_add_circuit);
}

/// Benchmarks the current Midnight-backed BN254 Fp2 multiplication circuit.
pub fn bench_fp2_mul(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "field", "bench_fp2_mul", run_fp2_mul_circuit);
}

/// Benchmarks the current Midnight-backed BN254 Fp2 square circuit.
pub fn bench_fp2_square(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "field", "bench_fp2_square", run_fp2_square_circuit);
}

/// Benchmarks the current Midnight-backed BN254 Fp6 addition circuit.
pub fn bench_fp6_add(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "field", "bench_fp6_add", run_fp6_add_circuit);
}

/// Benchmarks the current Midnight-backed BN254 Fp6 multiplication circuit.
pub fn bench_fp6_mul(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "field", "bench_fp6_mul", run_fp6_mul_circuit);
}

/// Benchmarks the current Midnight-backed BN254 Fp6 square circuit.
pub fn bench_fp6_square(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "field", "bench_fp6_square", run_fp6_square_circuit);
}

/// Benchmarks the current Midnight-backed BN254 Fp12 addition circuit.
pub fn bench_fp12_add(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "field", "bench_fp12_add", run_fp12_add_circuit);
}

/// Benchmarks the current Midnight-backed BN254 Fp12 multiplication circuit.
pub fn bench_fp12_mul(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "field", "bench_fp12_mul", run_fp12_mul_circuit);
}

/// Benchmarks the current Midnight-backed BN254 Fp12 square circuit.
pub fn bench_fp12_square(criterion: &mut Criterion) {
  bench_verified_sample(criterion, "field", "bench_fp12_square", run_fp12_square_circuit);
}

/// Benchmarks the current Midnight-backed BN254 Fp12 cyclotomic-square circuit.
pub fn bench_fp12_cyclotomic_square(criterion: &mut Criterion) {
  bench_verified_sample(
    criterion,
    "field",
    "bench_fp12_cyclotomic_square",
    run_fp12_cyclotomic_square_circuit,
  );
}
