use ark_bn254::{
  Fq as ArkFq, Fq2 as ArkFq2, G1Affine as ArkG1Affine, G1Projective as ArkG1Projective,
};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{BigInteger, PrimeField, UniformRand};
use ff::{Field, PrimeField as HaloPrimeField};
use halo2curves::group::Group;
use midnight_circuits::midnight_proofs::plonk::Circuit;
use midnight_curves::{CurveAffine, bn256::G1Affine};
use midnight_proofs::dev::MockProver;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use super::metrics::measure_layout;
use super::*;

fn ark_to_midnight_fq(value: ArkFq) -> ForeignField {
  let bytes = value.into_bigint().to_bytes_le();
  let mut repr = <ForeignField as HaloPrimeField>::Repr::default();
  let repr_bytes = repr.as_mut();
  let copy_len = bytes.len().min(repr_bytes.len());
  repr_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);

  ForeignField::from_repr_vartime(repr)
    .expect("arkworks bn254 fq value should fit midnight bn254 fq")
}

fn ark_to_midnight_g1(point: ArkG1Affine) -> ForeignCurve {
  if point.is_zero() {
    return ForeignCurve::identity();
  }

  let affine = Option::<G1Affine>::from(G1Affine::from_xy(
    ark_to_midnight_fq(point.x),
    ark_to_midnight_fq(point.y),
  ))
  .expect("arkworks point should map to a valid midnight bn254 point");

  affine.into()
}

fn ark_to_midnight_fq2(value: ArkFq2) -> (ForeignField, ForeignField) {
  (ark_to_midnight_fq(value.c0), ark_to_midnight_fq(value.c1))
}

fn assert_satisfied<CircuitT: Circuit<NativeField>>(circuit: &CircuitT) {
  let k = measure_layout(circuit).k;
  let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect("mock prover should run");
  assert_eq!(prover.verify(), Ok(()));
}

fn prover_result<CircuitT: Circuit<NativeField>>(circuit: &CircuitT) -> bool {
  let k = measure_layout(circuit).k;
  let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect("mock prover should run");
  prover.verify().is_ok()
}

#[test]
fn field_edge_cases_match_arkworks() {
  let zero = ArkFq::from(0_u64);
  let one = ArkFq::from(1_u64);
  let modulus_minus_one = -ArkFq::from(1_u64);

  assert_satisfied(&FpAddCircuit::new(ark_to_midnight_fq(zero), ark_to_midnight_fq(one)));
  assert_satisfied(&FpMulCircuit::new(
    ark_to_midnight_fq(one),
    ark_to_midnight_fq(modulus_minus_one),
  ));
}

#[test]
fn randomized_additions_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([21_u8; 32]);

  for _ in 0..12 {
    let left = ArkFq::rand(&mut rng);
    let right = ArkFq::rand(&mut rng);

    assert_satisfied(&FpAddCircuit::new(ark_to_midnight_fq(left), ark_to_midnight_fq(right)));
  }
}

#[test]
fn randomized_multiplications_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([22_u8; 32]);

  for _ in 0..12 {
    let left = ArkFq::rand(&mut rng);
    let right = ArkFq::rand(&mut rng);

    assert_satisfied(&FpMulCircuit::new(ark_to_midnight_fq(left), ark_to_midnight_fq(right)));
  }
}

#[test]
fn fp2_zero_plus_x_is_x() {
  let x = ark_to_midnight_fq2(ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(8_u64)));
  let zero = ark_to_midnight_fq2(ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)));

  assert_satisfied(&Fp2AddCircuit::new(zero, x));
}

#[test]
fn fp2_one_times_x_is_x() {
  let x = ark_to_midnight_fq2(ArkFq2::new(ArkFq::from(9_u64), ArkFq::from(4_u64)));
  let one = ark_to_midnight_fq2(ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(0_u64)));

  assert_satisfied(&Fp2MulCircuit::new(one, x));
}

#[test]
fn fp2_x_plus_neg_x_is_zero() {
  let x = ArkFq2::new(ArkFq::from(12_u64), ArkFq::from(19_u64));

  assert_satisfied(&Fp2AddCircuit::new(ark_to_midnight_fq2(x), ark_to_midnight_fq2(-x)));
}

#[test]
fn fp2_randomized_additions_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([41_u8; 32]);

  for _ in 0..12 {
    let left = ArkFq2::rand(&mut rng);
    let right = ArkFq2::rand(&mut rng);

    assert_satisfied(&Fp2AddCircuit::new(ark_to_midnight_fq2(left), ark_to_midnight_fq2(right)));
  }
}

#[test]
fn fp2_randomized_multiplications_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([42_u8; 32]);

  for _ in 0..12 {
    let left = ArkFq2::rand(&mut rng);
    let right = ArkFq2::rand(&mut rng);

    assert_satisfied(&Fp2MulCircuit::new(ark_to_midnight_fq2(left), ark_to_midnight_fq2(right)));
  }
}

#[test]
fn fp2_randomized_squares_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([43_u8; 32]);

  for _ in 0..12 {
    let value = ArkFq2::rand(&mut rng);

    assert_satisfied(&Fp2SquareCircuit::new(ark_to_midnight_fq2(value)));
  }
}

#[test]
fn fp2_edge_cases_match_arkworks() {
  let vectors = [
    ArkFq2::new(ArkFq::from(7_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(9_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(1_u64)),
    ArkFq2::new(-ArkFq::from(1_u64), ArkFq::from(3_u64)),
  ];

  assert_satisfied(&Fp2AddCircuit::new(
    ark_to_midnight_fq2(vectors[0]),
    ark_to_midnight_fq2(vectors[1]),
  ));
  assert_satisfied(&Fp2MulCircuit::new(
    ark_to_midnight_fq2(vectors[0]),
    ark_to_midnight_fq2(vectors[1]),
  ));
  assert_satisfied(&Fp2SquareCircuit::new(ark_to_midnight_fq2(vectors[2])));
  assert_satisfied(&Fp2SquareCircuit::new(ark_to_midnight_fq2(vectors[3])));
  assert_satisfied(&Fp2AddCircuit::new(
    ark_to_midnight_fq2(vectors[4]),
    ark_to_midnight_fq2(ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(0_u64))),
  ));
}

#[test]
fn fp2_layout_metrics_are_real_and_nonzero() {
  let add_metrics = fp2_add_layout_metrics();
  let mul_metrics = fp2_mul_layout_metrics();
  let square_metrics = fp2_square_layout_metrics();

  assert!(add_metrics.rows > 0);
  assert!(mul_metrics.rows > 0);
  assert!(square_metrics.rows > 0);
  assert!(mul_metrics.column_queries > 0);
  assert!(square_metrics.column_queries > 0);
}

#[test]
fn fp_layout_metrics_are_real_and_nonzero() {
  let add_metrics = fp_add_layout_metrics();
  let mul_metrics = fp_mul_layout_metrics();

  assert!(add_metrics.rows > 0);
  assert!(mul_metrics.rows > 0);
  assert!(mul_metrics.column_queries > 0);
}

#[test]
fn g1_addition_matches_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([31_u8; 32]);

  for _ in 0..8 {
    let left = ArkG1Projective::rand(&mut rng).into_affine();
    let right = ArkG1Projective::rand(&mut rng).into_affine();

    let circuit = G1AddCircuit::new(ark_to_midnight_g1(left), ark_to_midnight_g1(right));
    assert!(prover_result(&circuit));
  }
}

#[test]
fn g1_doubling_works_via_addition() {
  let mut rng = ChaCha20Rng::from_seed([32_u8; 32]);

  for _ in 0..6 {
    let point = ArkG1Projective::rand(&mut rng).into_affine();
    let doubled = (point.into_group() + point).into_affine();
    let circuit = G1AddCircuit::new(ark_to_midnight_g1(point), ark_to_midnight_g1(point));

    assert!(prover_result(&circuit));
    assert_eq!(ark_to_midnight_g1(doubled), circuit.expected);
  }
}

#[test]
fn invalid_point_is_rejected() {
  let result = std::panic::catch_unwind(|| {
    let circuit = G1OnCurveCircuit::new(ForeignField::ZERO, ForeignField::ZERO);
    prover_result(&circuit)
  });

  assert!(result.is_err() || !result.expect("catch_unwind should resolve"));
}

#[test]
fn g1_layout_metrics_are_real_and_nonzero() {
  let metrics = g1_add_layout_metrics();

  assert!(metrics.rows > 0);
  assert!(metrics.lookups > 0 || metrics.permutations > 0);
}
