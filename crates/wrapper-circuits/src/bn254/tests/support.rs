use ark_bn254::{
  Bn254 as ArkBn254, Config as ArkBn254Config, Fq as ArkFq, Fq2 as ArkFq2, Fq6 as ArkFq6,
  Fq12 as ArkFq12, G1Affine as ArkG1Affine, G2Affine as ArkG2Affine, g2,
};
use ark_ec::{
  AdditiveGroup, AffineRepr, CurveGroup, models::bn::G2Prepared as ArkPreparedG2,
  models::short_weierstrass::SWCurveConfig, pairing::Pairing,
};
use ark_ff::{Field as ArkField, UniformRand};
use midnight_circuits::midnight_proofs::{circuit::Value, plonk::Circuit};
use midnight_proofs::dev::MockProver;
use rand::RngCore;

use super::*;
use crate::bn254::host::Fp12Value;
use crate::bn254::metrics::measure_layout;
use crate::test_support::{
  ark_to_midnight_fq as shared_ark_to_midnight_fq, ark_to_midnight_g1 as shared_ark_to_midnight_g1,
};

pub(crate) type Fp2AssignedValue = (Value<ForeignField>, Value<ForeignField>);
pub(crate) type Fp6ConstantValue =
  ((ForeignField, ForeignField), (ForeignField, ForeignField), (ForeignField, ForeignField));
pub(crate) type Fp12ConstantValue = (Fp6ConstantValue, Fp6ConstantValue);
pub(crate) type G2AssignedValue = (Fp2AssignedValue, Fp2AssignedValue);
pub(crate) type G2ConstantValue = ((ForeignField, ForeignField), (ForeignField, ForeignField));
pub(crate) type G2MillerPointConstantValue =
  ((ForeignField, ForeignField), (ForeignField, ForeignField), (ForeignField, ForeignField));
pub(crate) type G2LineCoeffsConstantValue =
  ((ForeignField, ForeignField), (ForeignField, ForeignField), (ForeignField, ForeignField));
pub(crate) type PairingTermConstantValue =
  ((ForeignField, ForeignField), ((ForeignField, ForeignField), (ForeignField, ForeignField)));

#[derive(Clone, Copy, Debug)]
pub(crate) struct ArkG2MillerPoint {
  pub(crate) x: ArkFq2,
  pub(crate) y: ArkFq2,
  pub(crate) z: ArkFq2,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ArkG2LineCoeffs {
  pub(crate) constant: ArkFq2,
  pub(crate) x_scale: ArkFq2,
  pub(crate) vw: ArkFq2,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ArkMillerStep {
  Double(ArkG2LineCoeffs),
  Add(ArkG2LineCoeffs),
}

pub(crate) fn ark_to_midnight_fq(value: ArkFq) -> ForeignField {
  shared_ark_to_midnight_fq(value)
}

pub(crate) fn ark_to_midnight_g1(point: ArkG1Affine) -> ForeignCurve {
  shared_ark_to_midnight_g1(point)
}

pub(crate) fn ark_to_midnight_fq2(value: ArkFq2) -> (ForeignField, ForeignField) {
  (ark_to_midnight_fq(value.c0), ark_to_midnight_fq(value.c1))
}

pub(crate) fn ark_to_midnight_fq6(value: ArkFq6) -> Fp6ConstantValue {
  (ark_to_midnight_fq2(value.c0), ark_to_midnight_fq2(value.c1), ark_to_midnight_fq2(value.c2))
}

pub(crate) fn ark_to_midnight_fq12(value: &ArkFq12) -> Fp12ConstantValue {
  (ark_to_midnight_fq6(value.c0), ark_to_midnight_fq6(value.c1))
}

pub(crate) fn fp12_constant_witness(value: Fp12ConstantValue) -> Fp12Value {
  (
    (
      (Value::known(value.0.0.0), Value::known(value.0.0.1)),
      (Value::known(value.0.1.0), Value::known(value.0.1.1)),
      (Value::known(value.0.2.0), Value::known(value.0.2.1)),
    ),
    (
      (Value::known(value.1.0.0), Value::known(value.1.0.1)),
      (Value::known(value.1.1.0), Value::known(value.1.1.1)),
      (Value::known(value.1.2.0), Value::known(value.1.2.1)),
    ),
  )
}

pub(crate) fn assign_fixed_fp12(
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  value: Fp12ConstantValue,
) -> Result<AssignedFp12, Error> {
  let witness = fp12_constant_witness(value);
  AssignedFp12::assign(chip, layouter, witness.0, witness.1)
}

pub(crate) fn ark_zero_fq2() -> ArkFq2 {
  ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64))
}

pub(crate) fn ark_zero_fq6() -> ArkFq6 {
  ArkFq6::new(ark_zero_fq2(), ark_zero_fq2(), ark_zero_fq2())
}

pub(crate) fn ark_one_fq6() -> ArkFq6 {
  ArkFq6::new(ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(0_u64)), ark_zero_fq2(), ark_zero_fq2())
}

pub(crate) fn ark_to_assigned_g2_coords(
  point: ArkG2Affine,
) -> ((ForeignField, ForeignField), (ForeignField, ForeignField)) {
  assert!(!point.is_zero(), "this narrow G2 affine slice does not support infinity");
  (ark_to_midnight_fq2(point.x), ark_to_midnight_fq2(point.y))
}

pub(crate) fn ark_to_miller_point_constant(point: ArkG2MillerPoint) -> G2MillerPointConstantValue {
  (ark_to_midnight_fq2(point.x), ark_to_midnight_fq2(point.y), ark_to_midnight_fq2(point.z))
}

pub(crate) fn ark_scale_fq2(mut value: ArkFq2, scalar: ArkFq) -> ArkFq2 {
  value.mul_assign_by_fp(&scalar);
  value
}

pub(crate) fn ark_to_line_coeffs_constant(line: ArkG2LineCoeffs) -> G2LineCoeffsConstantValue {
  (
    ark_to_midnight_fq2(line.constant),
    ark_to_midnight_fq2(line.x_scale),
    ark_to_midnight_fq2(line.vw),
  )
}

pub(crate) fn ark_miller_point_from_affine(point: ArkG2Affine) -> ArkG2MillerPoint {
  ArkG2MillerPoint {
    x: point.x,
    y: point.y,
    z: ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(0_u64)),
  }
}

pub(crate) fn ark_miller_point_to_affine(point: ArkG2MillerPoint) -> ArkG2Affine {
  let z_inv = point.z.inverse().expect("test Miller-step point should remain non-identity");
  ArkG2Affine::new_unchecked(point.x * z_inv, point.y * z_inv)
}

pub(crate) fn ark_double_with_line(
  mut point: ArkG2MillerPoint,
) -> (ArkG2MillerPoint, ArkG2LineCoeffs) {
  let two_inv = ArkFq::from(2_u64).inverse().expect("hard-coded two should be invertible");
  let xy_half = ark_scale_fq2(point.x * point.y, two_inv);
  let y_square = point.y.square();
  let z_square = point.z.square();
  let twist_times_three_z_square = g2::Config::COEFF_B * (z_square.double() + z_square);
  let triple_twist_term = twist_times_three_z_square.double() + twist_times_three_z_square;
  let average_y_square_and_twist = ark_scale_fq2(y_square + triple_twist_term, two_inv);
  let y_plus_z_cross = (point.y + point.z).square() - (y_square + z_square);
  let vertical_term = twist_times_three_z_square - y_square;
  let x_square = point.x.square();
  let twist_term_square = twist_times_three_z_square.square();

  point.x = xy_half * (y_square - triple_twist_term);
  point.y = average_y_square_and_twist.square() - (twist_term_square.double() + twist_term_square);
  point.z = y_square * y_plus_z_cross;

  (
    point,
    ArkG2LineCoeffs {
      constant: -y_plus_z_cross,
      x_scale: x_square.double() + x_square,
      vw: vertical_term,
    },
  )
}

pub(crate) fn ark_mixed_add_with_line(
  mut point: ArkG2MillerPoint,
  addend: ArkG2Affine,
) -> (ArkG2MillerPoint, ArkG2LineCoeffs) {
  let theta = point.y - addend.y * point.z;
  let lambda = point.x - addend.x * point.z;
  let theta_square = theta.square();
  let lambda_square = lambda.square();
  let lambda_cubed = lambda * lambda_square;
  let z_times_theta_square = point.z * theta_square;
  let x_times_lambda_square = point.x * lambda_square;
  let next_x_intermediate = lambda_cubed + z_times_theta_square - x_times_lambda_square.double();

  point.x = lambda * next_x_intermediate;
  point.y = theta * (x_times_lambda_square - next_x_intermediate) - lambda_cubed * point.y;
  point.z *= lambda_cubed;

  (
    point,
    ArkG2LineCoeffs { constant: lambda, x_scale: -theta, vw: theta * addend.x - lambda * addend.y },
  )
}

pub(crate) fn ark_line_evaluation(line: ArkG2LineCoeffs, g1: ArkG1Affine) -> ArkFq12 {
  ArkFq12::new(
    ArkFq6::new(ark_scale_fq2(line.constant, g1.y), ark_zero_fq2(), ark_zero_fq2()),
    ArkFq6::new(ark_scale_fq2(line.x_scale, g1.x), line.vw, ark_zero_fq2()),
  )
}

pub(crate) fn ark_miller_loop_accumulate(steps: &[ArkMillerStep], g1: ArkG1Affine) -> ArkFq12 {
  let mut accumulator = ArkFq12::new(ark_one_fq6(), ark_zero_fq6());

  for step in steps {
    if matches!(step, ArkMillerStep::Double(_)) {
      accumulator = accumulator.square();
    }

    let line = match step {
      ArkMillerStep::Double(line) | ArkMillerStep::Add(line) => *line,
    };
    accumulator *= ark_line_evaluation(line, g1);
  }

  accumulator
}

pub(crate) fn ark_bn254_prepared_miller_steps(g2: ArkG2Affine) -> Vec<ArkMillerStep> {
  let prepared: ArkPreparedG2<ArkBn254Config> = g2.into();
  let schedule = Bn254MillerSchedule::bn254();

  assert_eq!(prepared.ell_coeffs.len(), schedule.steps.len());

  schedule
    .steps
    .iter()
    .zip(prepared.ell_coeffs)
    .map(|(step, coeff)| {
      let line = ArkG2LineCoeffs { constant: coeff.0, x_scale: coeff.1, vw: coeff.2 };
      match step {
        Bn254MillerScheduleStep::Double => ArkMillerStep::Double(line),
        Bn254MillerScheduleStep::Add(_) => ArkMillerStep::Add(line),
      }
    })
    .collect()
}

pub(crate) fn ark_bn254_miller_loop_accumulate(g2: ArkG2Affine, g1: ArkG1Affine) -> ArkFq12 {
  ark_miller_loop_accumulate(&ark_bn254_prepared_miller_steps(g2), g1)
}

pub(crate) fn ark_bn254_final_exponentiation(value: ArkFq12) -> ArkFq12 {
  ArkBn254::final_exponentiation(ark_ec::pairing::MillerLoopOutput(value))
    .expect("nonzero BN254 Miller output should admit final exponentiation")
    .0
}

pub(crate) fn ark_bn254_pairing(g1: ArkG1Affine, g2: ArkG2Affine) -> ArkFq12 {
  ArkBn254::pairing(g1, g2).0
}

pub(crate) fn ark_bn254_multi_miller_loop_product(terms: &[(ArkG1Affine, ArkG2Affine)]) -> ArkFq12 {
  terms.iter().fold(ArkFq12::ONE, |accumulator, (g1, g2)| {
    accumulator * ark_bn254_miller_loop_accumulate(*g2, *g1)
  })
}

pub(crate) fn ark_bn254_pairing_product(terms: &[(ArkG1Affine, ArkG2Affine)]) -> ArkFq12 {
  ark_bn254_final_exponentiation(ark_bn254_multi_miller_loop_product(terms))
}

pub(crate) fn ark_bn254_pairing_check(terms: &[(ArkG1Affine, ArkG2Affine)]) -> bool {
  ark_bn254_pairing_product(terms) == ArkFq12::ONE
}

pub(crate) fn random_nonzero_g1_affine(rng: &mut impl RngCore) -> ArkG1Affine {
  loop {
    let candidate = ark_bn254::G1Projective::rand(rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  }
}

pub(crate) fn random_nonzero_g2_affine(rng: &mut impl RngCore) -> ArkG2Affine {
  loop {
    let candidate = ark_bn254::G2Projective::rand(rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  }
}

pub(crate) fn random_supported_mixed_add_fixture(
  rng: &mut impl RngCore,
) -> (ArkG2Affine, ArkG2Affine, ArkG2MillerPoint) {
  loop {
    let seed_point = random_nonzero_g2_affine(rng);
    let addend = random_nonzero_g2_affine(rng);
    let doubled_state = ark_double_with_line(ark_miller_point_from_affine(seed_point)).0;
    let current_affine = ark_miller_point_to_affine(doubled_state);

    if addend != current_affine && addend != -current_affine {
      break (seed_point, addend, doubled_state);
    }
  }
}

pub(crate) fn ark_pairing_terms_to_constants(
  terms: &[(ArkG1Affine, ArkG2Affine)],
) -> Vec<PairingTermConstantValue> {
  terms
    .iter()
    .map(|(g1, g2)| {
      ((ark_to_midnight_fq(g1.x), ark_to_midnight_fq(g1.y)), ark_to_assigned_g2_coords(*g2))
    })
    .collect()
}

pub(crate) fn ark_generator_double_line_fixture()
-> (ArkG2Affine, ArkG1Affine, ArkG2MillerPoint, ArkG2LineCoeffs, ArkFq12) {
  let g2 = ArkG2Affine::generator();
  let g1 = ArkG1Affine::generator();
  let (next_state, line) = ark_double_with_line(ark_miller_point_from_affine(g2));
  let value = ark_line_evaluation(line, g1);

  (g2, g1, next_state, line, value)
}

pub(crate) fn ark_generator_double_add_fixture()
-> (ArkG2Affine, ArkG1Affine, ArkG2MillerPoint, ArkG2LineCoeffs, ArkG2LineCoeffs, ArkFq12) {
  let g2 = ArkG2Affine::generator();
  let g1 = ArkG1Affine::generator();
  let (doubled_state, double_line) = ark_double_with_line(ark_miller_point_from_affine(g2));
  let (_, add_line) = ark_mixed_add_with_line(doubled_state, g2);
  let value = ark_miller_loop_accumulate(
    &[ArkMillerStep::Double(double_line), ArkMillerStep::Add(add_line)],
    g1,
  );

  (g2, g1, doubled_state, double_line, add_line, value)
}

pub(crate) fn assert_satisfied<CircuitT: Circuit<NativeField>>(circuit: &CircuitT) {
  let k = measure_layout(circuit).k;
  let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect("mock prover should run");
  assert_eq!(prover.verify(), Ok(()));
}

pub(crate) fn prover_result<CircuitT: Circuit<NativeField>>(circuit: &CircuitT) -> bool {
  let k = measure_layout(circuit).k;
  let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect("mock prover should run");
  prover.verify().is_ok()
}
