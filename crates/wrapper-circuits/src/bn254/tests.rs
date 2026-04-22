use ark_bn254::{
  Fq as ArkFq, Fq2 as ArkFq2, Fq6 as ArkFq6, Fq6Config as ArkFq6Config, Fq12 as ArkFq12,
  Fq12Config as ArkFq12Config, G1Affine as ArkG1Affine, G1Projective as ArkG1Projective,
  G2Affine as ArkG2Affine, G2Projective as ArkG2Projective, g2,
};
use ark_ec::{AffineRepr, CurveGroup, PrimeGroup, models::short_weierstrass::SWCurveConfig};
use ark_ff::{Field as ArkField, Fp6Config, Fp12Config, UniformRand};
use ff::Field;
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use super::test_support::{
  ArkMillerStep, Fp12ConstantValue, G2AssignedValue, G2ConstantValue,
  ark_bn254_final_exponentiation, ark_bn254_miller_loop_accumulate,
  ark_bn254_multi_miller_loop_product, ark_bn254_pairing, ark_bn254_pairing_check,
  ark_bn254_pairing_product, ark_bn254_prepared_miller_steps, ark_double_with_line,
  ark_generator_double_add_fixture, ark_generator_double_line_fixture, ark_line_evaluation,
  ark_miller_point_from_affine, ark_miller_point_to_affine, ark_mixed_add_with_line, ark_one_fq6,
  ark_to_assigned_g2_coords, ark_to_line_coeffs_constant, ark_to_midnight_fq, ark_to_midnight_fq2,
  ark_to_midnight_fq6, ark_to_midnight_fq12, ark_to_midnight_g1, ark_to_miller_point_constant,
  ark_zero_fq6, assert_satisfied, prover_result,
};
use super::*;
use crate::bn254::g2::{
  MillerAccumulatorMulByLineCircuit, MillerAccumulatorMulByLineSparseCircuit,
};

#[derive(Clone, Debug)]
struct G2EqualityCircuit {
  left: G2AssignedValue,
  right: G2AssignedValue,
}

impl G2EqualityCircuit {
  fn new(left: G2ConstantValue, right: G2ConstantValue) -> Self {
    Self {
      left: (
        (Value::known(left.0.0), Value::known(left.0.1)),
        (Value::known(left.1.0), Value::known(left.1.1)),
      ),
      right: (
        (Value::known(right.0.0), Value::known(right.0.1)),
        (Value::known(right.1.0), Value::known(right.1.1)),
      ),
    }
  }
}

impl Circuit<NativeField> for G2EqualityCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      left: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      right: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254FieldChip::new(&config);
    let left = AssignedG2Affine::assign(&chip, &mut layouter, self.left.0, self.left.1)?;
    let right = AssignedG2Affine::assign(&chip, &mut layouter, self.right.0, self.right.1)?;
    left.assert_on_curve(&chip, &mut layouter)?;
    right.assert_on_curve(&chip, &mut layouter)?;
    left.assert_equal(&chip, &mut layouter, &right)?;
    chip.load(&mut layouter)
  }
}

#[derive(Clone, Debug)]
struct MillerAccumulatorOneCircuit;

impl Circuit<NativeField> for MillerAccumulatorOneCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254FieldChip::new(&config);
    let accumulator = AssignedMillerAccumulator::one(&chip, &mut layouter)?;
    let expected = AssignedFp12::one(&chip, &mut layouter)?;
    accumulator.f.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}

#[derive(Clone, Debug)]
struct MillerAccumulatorMulByEvaluatedLineCircuit {
  initial: Fp12ConstantValue,
  evaluated_line: Fp12ConstantValue,
  expected: Fp12ConstantValue,
}

impl MillerAccumulatorMulByEvaluatedLineCircuit {
  fn new(initial: &ArkFq12, evaluated_line: &ArkFq12, expected: &ArkFq12) -> Self {
    Self {
      initial: ark_to_midnight_fq12(initial),
      evaluated_line: ark_to_midnight_fq12(evaluated_line),
      expected: ark_to_midnight_fq12(expected),
    }
  }
}

impl Circuit<NativeField> for MillerAccumulatorMulByEvaluatedLineCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { initial: self.initial, evaluated_line: self.evaluated_line, expected: self.expected }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254FieldChip::new(&config);
    let initial = AssignedFp12::assign(
      &chip,
      &mut layouter,
      (
        (Value::known(self.initial.0.0.0), Value::known(self.initial.0.0.1)),
        (Value::known(self.initial.0.1.0), Value::known(self.initial.0.1.1)),
        (Value::known(self.initial.0.2.0), Value::known(self.initial.0.2.1)),
      ),
      (
        (Value::known(self.initial.1.0.0), Value::known(self.initial.1.0.1)),
        (Value::known(self.initial.1.1.0), Value::known(self.initial.1.1.1)),
        (Value::known(self.initial.1.2.0), Value::known(self.initial.1.2.1)),
      ),
    )?;
    let evaluated_line = AssignedFp12::assign(
      &chip,
      &mut layouter,
      (
        (Value::known(self.evaluated_line.0.0.0), Value::known(self.evaluated_line.0.0.1)),
        (Value::known(self.evaluated_line.0.1.0), Value::known(self.evaluated_line.0.1.1)),
        (Value::known(self.evaluated_line.0.2.0), Value::known(self.evaluated_line.0.2.1)),
      ),
      (
        (Value::known(self.evaluated_line.1.0.0), Value::known(self.evaluated_line.1.0.1)),
        (Value::known(self.evaluated_line.1.1.0), Value::known(self.evaluated_line.1.1.1)),
        (Value::known(self.evaluated_line.1.2.0), Value::known(self.evaluated_line.1.2.1)),
      ),
    )?;
    let expected = AssignedFp12::assign(
      &chip,
      &mut layouter,
      (
        (Value::known(self.expected.0.0.0), Value::known(self.expected.0.0.1)),
        (Value::known(self.expected.0.1.0), Value::known(self.expected.0.1.1)),
        (Value::known(self.expected.0.2.0), Value::known(self.expected.0.2.1)),
      ),
      (
        (Value::known(self.expected.1.0.0), Value::known(self.expected.1.0.1)),
        (Value::known(self.expected.1.1.0), Value::known(self.expected.1.1.1)),
        (Value::known(self.expected.1.2.0), Value::known(self.expected.1.2.1)),
      ),
    )?;

    let mut accumulator = AssignedMillerAccumulator::new(initial);
    accumulator.mul_by_evaluated_line(&chip, &mut layouter, &evaluated_line)?;
    accumulator.f.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
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
fn fp6_nonresidue_matches_arkworks() {
  assert_eq!(fp6_nonresidue(), ark_to_midnight_fq2(ArkFq6Config::NONRESIDUE));
}

#[test]
fn fp12_nonresidue_matches_arkworks() {
  assert_eq!(fp12_nonresidue(), ark_to_midnight_fq6(ArkFq12Config::NONRESIDUE));
}

#[test]
fn fp6_zero_plus_x_is_x() {
  let x = ArkFq6::new(
    ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(8_u64)),
    ArkFq2::new(ArkFq::from(13_u64), ArkFq::from(21_u64)),
    ArkFq2::new(ArkFq::from(34_u64), ArkFq::from(55_u64)),
  );
  let zero = ArkFq6::new(
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
  );

  assert_satisfied(&Fp6AddCircuit::new(ark_to_midnight_fq6(zero), ark_to_midnight_fq6(x)));
}

#[test]
fn fp6_one_times_x_is_x() {
  let x = ArkFq6::new(
    ArkFq2::new(ArkFq::from(9_u64), ArkFq::from(4_u64)),
    ArkFq2::new(ArkFq::from(7_u64), ArkFq::from(3_u64)),
    ArkFq2::new(ArkFq::from(11_u64), ArkFq::from(6_u64)),
  );
  let one = ArkFq6::new(
    ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
  );

  assert_satisfied(&Fp6MulCircuit::new(ark_to_midnight_fq6(one), ark_to_midnight_fq6(x)));
}

#[test]
fn fp6_x_plus_neg_x_is_zero() {
  let x = ArkFq6::new(
    ArkFq2::new(ArkFq::from(12_u64), ArkFq::from(19_u64)),
    ArkFq2::new(ArkFq::from(2_u64), ArkFq::from(7_u64)),
    ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(14_u64)),
  );

  assert_satisfied(&Fp6AddCircuit::new(ark_to_midnight_fq6(x), ark_to_midnight_fq6(-x)));
}

#[test]
fn fp6_randomized_additions_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([61_u8; 32]);

  for _ in 0..10 {
    let left = ArkFq6::rand(&mut rng);
    let right = ArkFq6::rand(&mut rng);

    assert_satisfied(&Fp6AddCircuit::new(ark_to_midnight_fq6(left), ark_to_midnight_fq6(right)));
  }
}

#[test]
fn fp6_randomized_multiplications_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([62_u8; 32]);

  for _ in 0..10 {
    let left = ArkFq6::rand(&mut rng);
    let right = ArkFq6::rand(&mut rng);

    assert_satisfied(&Fp6MulCircuit::new(ark_to_midnight_fq6(left), ark_to_midnight_fq6(right)));
  }
}

#[test]
fn fp6_randomized_squares_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([63_u8; 32]);

  for _ in 0..10 {
    let value = ArkFq6::rand(&mut rng);

    assert_satisfied(&Fp6SquareCircuit::new(ark_to_midnight_fq6(value)));
  }
}

#[test]
fn fp6_edge_cases_match_arkworks() {
  let vectors = [
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(7_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(9_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(4_u64), ArkFq::from(6_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(1_u64)),
      ArkFq2::new(ArkFq::from(2_u64), ArkFq::from(3_u64)),
      ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(8_u64)),
    ),
  ];

  assert_satisfied(&Fp6AddCircuit::new(
    ark_to_midnight_fq6(vectors[0]),
    ark_to_midnight_fq6(vectors[1]),
  ));
  assert_satisfied(&Fp6MulCircuit::new(
    ark_to_midnight_fq6(vectors[0]),
    ark_to_midnight_fq6(vectors[3]),
  ));
  assert_satisfied(&Fp6MulCircuit::new(
    ark_to_midnight_fq6(vectors[1]),
    ark_to_midnight_fq6(vectors[3]),
  ));
  assert_satisfied(&Fp6MulCircuit::new(
    ark_to_midnight_fq6(vectors[2]),
    ark_to_midnight_fq6(vectors[3]),
  ));
  assert_satisfied(&Fp6SquareCircuit::new(ark_to_midnight_fq6(vectors[2])));
  assert_satisfied(&Fp6SquareCircuit::new(ark_to_midnight_fq6(vectors[3])));
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
fn fp6_layout_metrics_are_real_and_nonzero() {
  let add_metrics = fp6_add_layout_metrics();
  let mul_metrics = fp6_mul_layout_metrics();
  let square_metrics = fp6_square_layout_metrics();

  assert!(add_metrics.rows > 0);
  assert!(mul_metrics.rows > 0);
  assert!(square_metrics.rows > 0);
  assert!(add_metrics.column_queries > 0);
  assert!(mul_metrics.column_queries > 0);
  assert!(square_metrics.column_queries > 0);
}

#[test]
fn fp12_zero_plus_x_is_x() {
  let x = ArkFq12::new(
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(8_u64)),
      ArkFq2::new(ArkFq::from(13_u64), ArkFq::from(21_u64)),
      ArkFq2::new(ArkFq::from(34_u64), ArkFq::from(55_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(89_u64), ArkFq::from(144_u64)),
      ArkFq2::new(ArkFq::from(233_u64), ArkFq::from(377_u64)),
      ArkFq2::new(ArkFq::from(610_u64), ArkFq::from(987_u64)),
    ),
  );
  let zero = ArkFq12::new(ark_zero_fq6(), ark_zero_fq6());

  assert_satisfied(&Fp12AddCircuit::new(ark_to_midnight_fq12(&zero), ark_to_midnight_fq12(&x)));
}

#[test]
fn fp12_one_times_x_is_x() {
  let x = ArkFq12::new(
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(9_u64), ArkFq::from(4_u64)),
      ArkFq2::new(ArkFq::from(7_u64), ArkFq::from(3_u64)),
      ArkFq2::new(ArkFq::from(11_u64), ArkFq::from(6_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(10_u64), ArkFq::from(12_u64)),
      ArkFq2::new(ArkFq::from(14_u64), ArkFq::from(16_u64)),
      ArkFq2::new(ArkFq::from(18_u64), ArkFq::from(20_u64)),
    ),
  );
  let one = ArkFq12::new(ark_one_fq6(), ark_zero_fq6());

  assert_satisfied(&Fp12MulCircuit::new(ark_to_midnight_fq12(&one), ark_to_midnight_fq12(&x)));
}

#[test]
fn fp12_x_plus_neg_x_is_zero() {
  let x = ArkFq12::new(
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(12_u64), ArkFq::from(19_u64)),
      ArkFq2::new(ArkFq::from(2_u64), ArkFq::from(7_u64)),
      ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(14_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(22_u64), ArkFq::from(29_u64)),
      ArkFq2::new(ArkFq::from(31_u64), ArkFq::from(37_u64)),
      ArkFq2::new(ArkFq::from(41_u64), ArkFq::from(43_u64)),
    ),
  );

  let neg_x = -x;
  assert_satisfied(&Fp12AddCircuit::new(ark_to_midnight_fq12(&x), ark_to_midnight_fq12(&neg_x)));
}

#[test]
fn fp12_randomized_additions_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([71_u8; 32]);

  for _ in 0..10 {
    let left = ArkFq12::rand(&mut rng);
    let right = ArkFq12::rand(&mut rng);

    assert_satisfied(&Fp12AddCircuit::new(
      ark_to_midnight_fq12(&left),
      ark_to_midnight_fq12(&right),
    ));
  }
}

#[test]
fn fp12_randomized_multiplications_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([72_u8; 32]);

  for _ in 0..10 {
    let left = ArkFq12::rand(&mut rng);
    let right = ArkFq12::rand(&mut rng);

    assert_satisfied(&Fp12MulCircuit::new(
      ark_to_midnight_fq12(&left),
      ark_to_midnight_fq12(&right),
    ));
  }
}

#[test]
fn fp12_randomized_squares_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([73_u8; 32]);

  for _ in 0..10 {
    let value = ArkFq12::rand(&mut rng);

    assert_satisfied(&Fp12SquareCircuit::new(ark_to_midnight_fq12(&value)));
  }
}

#[test]
fn fp12_structured_cases_match_arkworks() {
  let c0_only = ArkFq12::new(
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(7_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(9_u64)),
      ArkFq2::new(ArkFq::from(4_u64), ArkFq::from(6_u64)),
    ),
    ark_zero_fq6(),
  );
  let c1_only = ArkFq12::new(
    ark_zero_fq6(),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(1_u64)),
      ArkFq2::new(ArkFq::from(2_u64), ArkFq::from(3_u64)),
      ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(8_u64)),
    ),
  );
  let mixed_small = ArkFq12::new(
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(2_u64)),
      ArkFq2::new(ArkFq::from(3_u64), ArkFq::from(5_u64)),
      ArkFq2::new(ArkFq::from(8_u64), ArkFq::from(13_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(21_u64), ArkFq::from(34_u64)),
      ArkFq2::new(ArkFq::from(55_u64), ArkFq::from(89_u64)),
      ArkFq2::new(ArkFq::from(144_u64), ArkFq::from(233_u64)),
    ),
  );

  assert_satisfied(&Fp12AddCircuit::new(
    ark_to_midnight_fq12(&c0_only),
    ark_to_midnight_fq12(&c1_only),
  ));
  assert_satisfied(&Fp12MulCircuit::new(
    ark_to_midnight_fq12(&c0_only),
    ark_to_midnight_fq12(&mixed_small),
  ));
  assert_satisfied(&Fp12MulCircuit::new(
    ark_to_midnight_fq12(&c1_only),
    ark_to_midnight_fq12(&mixed_small),
  ));
  assert_satisfied(&Fp12SquareCircuit::new(ark_to_midnight_fq12(&c0_only)));
  assert_satisfied(&Fp12SquareCircuit::new(ark_to_midnight_fq12(&c1_only)));
  assert_satisfied(&Fp12SquareCircuit::new(ark_to_midnight_fq12(&mixed_small)));
}

#[test]
fn fp12_layout_metrics_are_real_and_nonzero() {
  let add_metrics = fp12_add_layout_metrics();
  let mul_metrics = fp12_mul_layout_metrics();
  let square_metrics = fp12_square_layout_metrics();

  assert!(add_metrics.rows > 0);
  assert!(mul_metrics.rows > 0);
  assert!(square_metrics.rows > 0);
  assert!(add_metrics.column_queries > 0);
  assert!(mul_metrics.column_queries > 0);
  assert!(square_metrics.column_queries > 0);
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

#[test]
fn g2_curve_coeff_b_matches_arkworks() {
  assert_eq!(g2_curve_coeff_b(), ark_to_midnight_fq2(g2::Config::COEFF_B));
}

#[test]
fn g2_generator_is_on_curve() {
  let generator = ark_to_assigned_g2_coords(ArkG2Affine::generator());

  assert_satisfied(&G2OnCurveCircuit::new(generator.0, generator.1));
}

#[test]
fn random_valid_g2_points_pass_on_curve_checks() {
  let mut rng = ChaCha20Rng::from_seed([51_u8; 32]);

  for _ in 0..8 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    let point = ark_to_assigned_g2_coords(point);
    assert_satisfied(&G2OnCurveCircuit::new(point.0, point.1));
  }
}

#[test]
fn modified_g2_x_coordinates_are_rejected() {
  let point = ArkG2Affine::generator();
  let bad_x = ArkFq2::new(point.x.c0 + ArkFq::from(1_u64), point.x.c1);

  assert!(!prover_result(&G2OnCurveCircuit::new(
    ark_to_midnight_fq2(bad_x),
    ark_to_midnight_fq2(point.y),
  )));
}

#[test]
fn perturbed_g2_y_coordinates_are_rejected() {
  let point = ArkG2Affine::generator();
  let bad_y = ArkFq2::new(point.y.c0, point.y.c1 + ArkFq::from(1_u64));

  assert!(!prover_result(&G2OnCurveCircuit::new(
    ark_to_midnight_fq2(point.x),
    ark_to_midnight_fq2(bad_y),
  )));
}

#[test]
fn g2_negation_preserves_on_curve_validity() {
  let mut rng = ChaCha20Rng::from_seed([52_u8; 32]);

  for _ in 0..6 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    let negated = -point;
    assert_satisfied(&G2NegCircuit::new(
      ark_to_assigned_g2_coords(point),
      ark_to_assigned_g2_coords(negated),
    ));
  }
}

#[test]
fn g2_projective_identity_encoding_is_available() {
  assert_satisfied(&G2ProjectiveIdentityCircuit);
}

#[test]
fn g2_projective_from_affine_matches_the_same_affine_point() {
  let mut rng = ChaCha20Rng::from_seed([53_u8; 32]);

  for _ in 0..6 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    assert_satisfied(&G2ProjectiveFromAffineCircuit::new(ark_to_assigned_g2_coords(point)));
  }
}

#[test]
fn g2_projective_negation_matches_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([54_u8; 32]);

  for _ in 0..6 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    let negated = -point;
    assert_satisfied(&G2ProjectiveNegCircuit::new(
      ark_to_assigned_g2_coords(point),
      ark_to_assigned_g2_coords(negated),
    ));
  }
}

#[test]
fn g2_projective_doubling_matches_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([55_u8; 32]);

  for _ in 0..8 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    let doubled = (point.into_group() + point).into_affine();
    assert_satisfied(&G2ProjectiveDoubleCircuit::new(
      ark_to_assigned_g2_coords(point),
      ark_to_assigned_g2_coords(doubled),
    ));
  }
}

#[test]
fn g2_projective_addition_matches_arkworks_for_distinct_points() {
  let mut rng = ChaCha20Rng::from_seed([56_u8; 32]);

  for _ in 0..8 {
    let left = ArkG2Projective::rand(&mut rng).into_affine();
    let mut right = ArkG2Projective::rand(&mut rng).into_affine();

    if left.is_zero() || right.is_zero() {
      continue;
    }

    while right == left || right == -left {
      right = ArkG2Projective::rand(&mut rng).into_affine();
    }

    let expected = (left.into_group() + right).into_affine();
    assert_satisfied(&G2ProjectiveAddCircuit::new(
      ark_to_assigned_g2_coords(left),
      ark_to_assigned_g2_coords(right),
      ark_to_assigned_g2_coords(expected),
    ));
  }
}

#[test]
fn g2_projective_doubling_matches_generator_edge_case() {
  let generator = ArkG2Affine::generator();
  let expected = (generator.into_group() + generator).into_affine();

  assert_satisfied(&G2ProjectiveDoubleCircuit::new(
    ark_to_assigned_g2_coords(generator),
    ark_to_assigned_g2_coords(expected),
  ));
}

#[test]
fn g2_projective_addition_matches_generator_plus_double_generator() {
  let generator = ArkG2Affine::generator();
  let double_generator = (generator.into_group() + generator).into_affine();
  let expected = (generator.into_group() + double_generator).into_affine();

  assert_satisfied(&G2ProjectiveAddCircuit::new(
    ark_to_assigned_g2_coords(generator),
    ark_to_assigned_g2_coords(double_generator),
    ark_to_assigned_g2_coords(expected),
  ));
}

#[test]
fn g2_projective_addition_of_inverses_is_not_supported_in_this_slice() {
  let point = ArkG2Affine::generator();
  let negated = -point;

  assert!(!prover_result(&G2ProjectiveAddCircuit::new(
    ark_to_assigned_g2_coords(point),
    ark_to_assigned_g2_coords(negated),
    ark_to_assigned_g2_coords(point),
  )));
}

#[test]
fn g2_double_with_line_matches_arkworks_reference() {
  let mut rng = ChaCha20Rng::from_seed([57_u8; 32]);

  for _ in 0..8 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    let (next_point, line) = ark_double_with_line(ark_miller_point_from_affine(point));
    let expected_point = ark_miller_point_to_affine(next_point);

    assert_satisfied(&G2DoubleWithLineCircuit::new(
      ark_to_assigned_g2_coords(point),
      ark_to_assigned_g2_coords(expected_point),
      ark_to_line_coeffs_constant(line),
    ));
  }
}

#[test]
fn g2_double_with_line_matches_fixed_generator_fixture() {
  let (g2_point, _, next_state, line, _) = ark_generator_double_line_fixture();
  let expected_point = ark_miller_point_to_affine(next_state);

  assert_satisfied(&G2DoubleWithLineCircuit::new(
    ark_to_assigned_g2_coords(g2_point),
    ark_to_assigned_g2_coords(expected_point),
    ark_to_line_coeffs_constant(line),
  ));
}

#[test]
fn g2_mixed_add_with_line_matches_arkworks_reference() {
  let mut rng = ChaCha20Rng::from_seed([58_u8; 32]);

  for _ in 0..8 {
    let seed_point = ArkG2Projective::rand(&mut rng).into_affine();
    let addend = ArkG2Projective::rand(&mut rng).into_affine();

    if seed_point.is_zero() || addend.is_zero() {
      continue;
    }

    let doubled_state = ark_double_with_line(ark_miller_point_from_affine(seed_point)).0;
    let current_affine = ark_miller_point_to_affine(doubled_state);
    if addend == current_affine || addend == -current_affine {
      continue;
    }

    let (next_point, line) = ark_mixed_add_with_line(doubled_state, addend);
    let expected_point = ark_miller_point_to_affine(next_point);

    assert_satisfied(&G2MixedAddWithLineCircuit::new(
      ark_to_miller_point_constant(doubled_state),
      ark_to_assigned_g2_coords(addend),
      ark_to_assigned_g2_coords(expected_point),
      ark_to_line_coeffs_constant(line),
    ));
  }
}

#[test]
fn g2_mixed_add_with_line_matches_fixed_generator_fixture() {
  let (g2_point, _, doubled_state, _, add_line, _) = ark_generator_double_add_fixture();
  let (next_state, _) = ark_mixed_add_with_line(doubled_state, g2_point);
  let expected_point = ark_miller_point_to_affine(next_state);

  assert_satisfied(&G2MixedAddWithLineCircuit::new(
    ark_to_miller_point_constant(doubled_state),
    ark_to_assigned_g2_coords(g2_point),
    ark_to_assigned_g2_coords(expected_point),
    ark_to_line_coeffs_constant(add_line),
  ));
}

#[test]
fn g2_line_coeff_evaluation_matches_sparse_fp12_embedding() {
  let (_, g1_point, _, line, expected) = ark_generator_double_line_fixture();

  assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(
    ark_to_line_coeffs_constant(line),
    ark_to_midnight_fq(g1_point.x),
    ark_to_midnight_fq(g1_point.y),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
fn miller_accumulator_one_is_fp12_identity() {
  assert_satisfied(&MillerAccumulatorOneCircuit);
}

#[test]
fn miller_accumulator_square_matches_arkworks_reference() {
  let mut rng = ChaCha20Rng::from_seed([59_u8; 32]);

  for _ in 0..8 {
    let value = ArkFq12::rand(&mut rng);
    let expected = value.square();
    assert_satisfied(&MillerAccumulatorSquareCircuit::new(
      &ark_to_midnight_fq12(&value),
      &ark_to_midnight_fq12(&expected),
    ));
  }
}

#[test]
fn miller_accumulator_square_matches_fixed_generator_line_fixture() {
  let (_, _, _, _, value) = ark_generator_double_line_fixture();
  let expected = value.square();

  assert_satisfied(&MillerAccumulatorSquareCircuit::new(
    &ark_to_midnight_fq12(&value),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
fn miller_accumulator_mul_by_evaluated_line_matches_arkworks_reference() {
  let mut rng = ChaCha20Rng::from_seed([60_u8; 32]);

  for _ in 0..8 {
    let initial = ArkFq12::rand(&mut rng);
    let line_value = ArkFq12::rand(&mut rng);
    let expected = initial * line_value;

    assert_satisfied(&MillerAccumulatorMulByEvaluatedLineCircuit::new(
      &initial,
      &line_value,
      &expected,
    ));
  }
}

#[test]
fn miller_accumulator_mul_by_line_matches_arkworks_reference() {
  let mut rng = ChaCha20Rng::from_seed([61_u8; 32]);

  for _ in 0..8 {
    let g2_point = ArkG2Projective::rand(&mut rng).into_affine();
    let g1_point = ArkG1Projective::rand(&mut rng).into_affine();
    if g2_point.is_zero() || g1_point.is_zero() {
      continue;
    }

    let (_, line) = ark_double_with_line(ark_miller_point_from_affine(g2_point));
    let expected = ark_line_evaluation(line, g1_point);

    assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(
      ark_to_line_coeffs_constant(line),
      ark_to_midnight_fq(g1_point.x),
      ark_to_midnight_fq(g1_point.y),
      &ark_to_midnight_fq12(&expected),
    ));
  }
}

#[test]
fn miller_accumulator_mul_by_line_baseline_and_sparse_match_fixed_fixture() {
  let (_, g1_point, _, line, expected) = ark_generator_double_line_fixture();
  let line = ark_to_line_coeffs_constant(line);
  let g1_x = ark_to_midnight_fq(g1_point.x);
  let g1_y = ark_to_midnight_fq(g1_point.y);
  let expected = ark_to_midnight_fq12(&expected);

  assert_satisfied(&MillerAccumulatorMulByLineCircuit::new(line, g1_x, g1_y, &expected));
  assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(line, g1_x, g1_y, &expected));
}

#[test]
fn miller_accumulator_mul_by_line_baseline_and_sparse_match_randomized_fixtures() {
  let mut rng = ChaCha20Rng::from_seed([65_u8; 32]);

  for _ in 0..4 {
    let g2_point = ArkG2Projective::rand(&mut rng).into_affine();
    let g1_point = ArkG1Projective::rand(&mut rng).into_affine();
    if g2_point.is_zero() || g1_point.is_zero() {
      continue;
    }

    let (_, line) = ark_double_with_line(ark_miller_point_from_affine(g2_point));
    let expected = ark_line_evaluation(line, g1_point);
    let line = ark_to_line_coeffs_constant(line);
    let g1_x = ark_to_midnight_fq(g1_point.x);
    let g1_y = ark_to_midnight_fq(g1_point.y);
    let expected = ark_to_midnight_fq12(&expected);

    assert_satisfied(&MillerAccumulatorMulByLineCircuit::new(line, g1_x, g1_y, &expected));
    assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(line, g1_x, g1_y, &expected));
  }
}

#[test]
fn mixed_add_with_line_then_accumulate_matches_arkworks_reference() {
  let mut rng = ChaCha20Rng::from_seed([63_u8; 32]);

  for _ in 0..8 {
    let seed_point = ArkG2Projective::rand(&mut rng).into_affine();
    let addend = ArkG2Projective::rand(&mut rng).into_affine();
    let g1_point = ArkG1Projective::rand(&mut rng).into_affine();

    if seed_point.is_zero() || addend.is_zero() || g1_point.is_zero() {
      continue;
    }

    let doubled_state = ark_double_with_line(ark_miller_point_from_affine(seed_point)).0;
    let current_affine = ark_miller_point_to_affine(doubled_state);
    if addend == current_affine || addend == -current_affine {
      continue;
    }

    let (_, line) = ark_mixed_add_with_line(doubled_state, addend);
    let expected = ark_line_evaluation(line, g1_point);

    assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(
      ark_to_line_coeffs_constant(line),
      ark_to_midnight_fq(g1_point.x),
      ark_to_midnight_fq(g1_point.y),
      &ark_to_midnight_fq12(&expected),
    ));
  }
}

#[test]
fn miller_accumulator_sparse_and_generic_mul_by_line_paths_match_same_reference() {
  let g2_point = ArkG2Affine::generator();
  let g1_point = ArkG1Affine::generator();
  let (_, line) = ark_double_with_line(ark_miller_point_from_affine(g2_point));
  let expected = ark_line_evaluation(line, g1_point);
  let expected = ark_to_midnight_fq12(&expected);
  let line = ark_to_line_coeffs_constant(line);
  let g1_x = ark_to_midnight_fq(g1_point.x);
  let g1_y = ark_to_midnight_fq(g1_point.y);

  assert_satisfied(&MillerAccumulatorMulByLineCircuit::new(line, g1_x, g1_y, &expected));
  assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(line, g1_x, g1_y, &expected));
}

#[test]
fn bn254_miller_schedule_matches_expected_optimal_ate_shape() {
  let expected_loop_digits = [
    1, 0, 1, 0, 0, 0, -1, 0, -1, 0, 0, 0, -1, 0, 1, 0, -1, 0, 0, -1, 0, 0, 0, 0, 0, 1, 0, 0, -1, 0,
    1, 0, 0, -1, 0, 0, 0, 0, -1, 0, 1, 0, 0, 0, -1, 0, -1, 0, 0, 1, 0, 0, 0, -1, 0, 0, -1, 0, 1, 0,
    1, 0, 0, 0,
  ];
  let mut expected = Vec::new();
  for digit in expected_loop_digits {
    expected.push(Bn254MillerScheduleStep::Double);
    match digit {
      1 => expected.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::Base)),
      -1 => expected.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::NegBase)),
      0 => {}
      _ => unreachable!("test digits are ternary"),
    }
  }
  expected.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::FrobeniusQ1));
  expected.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::FrobeniusQ2NegY));

  let schedule = Bn254MillerSchedule::bn254();

  assert_eq!(bn254_ate_loop_count().len(), 65);
  assert_eq!(schedule.steps, expected);
  assert_eq!(
    schedule.steps.iter().filter(|step| matches!(step, Bn254MillerScheduleStep::Double)).count(),
    64,
  );
}

#[test]
fn bn254_prepared_schedule_matches_arkworks_prepared_coeffs() {
  let g2_point = ArkG2Affine::generator();
  let expected = ark_bn254_prepared_miller_steps(g2_point);
  let schedule = Bn254MillerSchedule::bn254();

  assert_eq!(expected.len(), schedule.steps.len());
  assert!(matches!(expected.first(), Some(ArkMillerStep::Double(_))));
  assert!(matches!(expected.last(), Some(ArkMillerStep::Add(_))));
}

#[test]
fn miller_loop_real_bn254_schedule_matches_arkworks_reference_on_generator() {
  let g1_point = ArkG1Affine::generator();
  let g2_point = ArkG2Affine::generator();
  let expected = ark_bn254_miller_loop_accumulate(g2_point, g1_point);

  assert_satisfied(&MillerLoopCircuit::new(
    (ark_to_midnight_fq(g1_point.x), ark_to_midnight_fq(g1_point.y)),
    ark_to_assigned_g2_coords(g2_point),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
fn miller_loop_real_bn254_schedule_matches_arkworks_reference() {
  let mut rng = ChaCha20Rng::from_seed([66_u8; 32]);

  let g1_point = loop {
    let candidate = ArkG1Projective::rand(&mut rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  };

  let base_point = loop {
    let candidate = ArkG2Projective::rand(&mut rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  };

  let expected = ark_bn254_miller_loop_accumulate(base_point, g1_point);

  assert_satisfied(&MillerLoopCircuit::new(
    (ark_to_midnight_fq(g1_point.x), ark_to_midnight_fq(g1_point.y)),
    ark_to_assigned_g2_coords(base_point),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
fn final_exponentiation_matches_arkworks_on_generator_miller_output() {
  let g1_point = ArkG1Affine::generator();
  let g2_point = ArkG2Affine::generator();
  let miller_output = ark_bn254_miller_loop_accumulate(g2_point, g1_point);
  let expected = ark_bn254_final_exponentiation(miller_output);

  assert_satisfied(&FinalExponentiationCircuit::new(
    &ark_to_midnight_fq12(&miller_output),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
fn final_exponentiation_matches_arkworks_on_deterministic_random_miller_outputs() {
  let mut rng = ChaCha20Rng::from_seed([67_u8; 32]);

  for _ in 0..3 {
    let g1_point = loop {
      let candidate = ArkG1Projective::rand(&mut rng).into_affine();
      if !candidate.is_zero() {
        break candidate;
      }
    };

    let g2_point = loop {
      let candidate = ArkG2Projective::rand(&mut rng).into_affine();
      if !candidate.is_zero() {
        break candidate;
      }
    };

    let miller_output = ark_bn254_miller_loop_accumulate(g2_point, g1_point);
    let expected = ark_bn254_final_exponentiation(miller_output);

    assert_satisfied(&FinalExponentiationCircuit::new(
      &ark_to_midnight_fq12(&miller_output),
      &ark_to_midnight_fq12(&expected),
    ));
  }
}

#[test]
fn miller_loop_then_final_exponentiation_matches_arkworks_pairing_on_generator() {
  let g1_point = ArkG1Affine::generator();
  let g2_point = ArkG2Affine::generator();
  let expected = ark_bn254_pairing(g1_point, g2_point);

  assert_satisfied(&PairingFinalExponentiationCircuit::new(
    (ark_to_midnight_fq(g1_point.x), ark_to_midnight_fq(g1_point.y)),
    ark_to_assigned_g2_coords(g2_point),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
fn miller_loop_then_final_exponentiation_matches_arkworks_pairing() {
  let mut rng = ChaCha20Rng::from_seed([68_u8; 32]);

  for _ in 0..2 {
    let g1_point = loop {
      let candidate = ArkG1Projective::rand(&mut rng).into_affine();
      if !candidate.is_zero() {
        break candidate;
      }
    };

    let g2_point = loop {
      let candidate = ArkG2Projective::rand(&mut rng).into_affine();
      if !candidate.is_zero() {
        break candidate;
      }
    };

    let expected = ark_bn254_pairing(g1_point, g2_point);

    assert_satisfied(&PairingFinalExponentiationCircuit::new(
      (ark_to_midnight_fq(g1_point.x), ark_to_midnight_fq(g1_point.y)),
      ark_to_assigned_g2_coords(g2_point),
      &ark_to_midnight_fq12(&expected),
    ));
  }
}

fn pairing_terms_to_constants(
  terms: &[(ArkG1Affine, ArkG2Affine)],
) -> Vec<((ForeignField, ForeignField), ((ForeignField, ForeignField), (ForeignField, ForeignField)))>
{
  terms
    .iter()
    .map(|(g1, g2)| {
      ((ark_to_midnight_fq(g1.x), ark_to_midnight_fq(g1.y)), ark_to_assigned_g2_coords(*g2))
    })
    .collect()
}

#[test]
fn multi_miller_loop_matches_product_of_individual_miller_outputs() {
  let g1 = ArkG1Affine::generator();
  let g2 = ArkG2Affine::generator();
  let two_g1 = (ArkG1Projective::generator() + ArkG1Projective::generator()).into_affine();
  let terms = [(g1, g2), (two_g1, g2)];
  let expected =
    ark_bn254_miller_loop_accumulate(g2, g1) * ark_bn254_miller_loop_accumulate(g2, two_g1);
  let actual = ark_bn254_multi_miller_loop_product(&terms);

  assert_eq!(actual, expected);
}

#[test]
fn pairing_check_one_term_matches_arkworks_negative_case() {
  let terms = [(ArkG1Affine::generator(), ArkG2Affine::generator())];
  assert!(!ark_bn254_pairing_check(&terms));

  let terms = pairing_terms_to_constants(&terms);
  assert_satisfied(&PairingCheckCircuit::new(&terms, false));
}

#[test]
fn pairing_check_two_term_inverse_cancellation_matches_arkworks() {
  let g2 = ArkG2Affine::generator();
  let g1 = ArkG1Affine::generator();
  let neg_g1 = (-ArkG1Projective::generator()).into_affine();
  let terms = [(g1, g2), (neg_g1, g2)];
  assert!(ark_bn254_pairing_check(&terms));
  assert_eq!(ark_bn254_pairing_product(&terms), ArkFq12::ONE);

  let terms = pairing_terms_to_constants(&terms);
  assert_satisfied(&PairingCheckCircuit::new(&terms, true));
}

#[test]
fn pairing_check_two_term_negative_matches_arkworks() {
  let g2 = ArkG2Affine::generator();
  let g1 = ArkG1Affine::generator();
  let two_g1 = (ArkG1Projective::generator() + ArkG1Projective::generator()).into_affine();
  let terms = [(g1, g2), (two_g1, g2)];
  assert!(!ark_bn254_pairing_check(&terms));

  let terms = pairing_terms_to_constants(&terms);
  assert_satisfied(&PairingCheckCircuit::new(&terms, false));
}

#[test]
fn pairing_check_three_term_cancellation_matches_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([69_u8; 32]);
  let q = loop {
    let candidate = ArkG2Projective::rand(&mut rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  };
  let p1 = loop {
    let candidate = ArkG1Projective::rand(&mut rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  };
  let p2 = loop {
    let candidate = ArkG1Projective::rand(&mut rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  };
  let p3 = (-(p1.into_group() + p2.into_group())).into_affine();
  let terms = [(p1, q), (p2, q), (p3, q)];
  assert!(ark_bn254_pairing_check(&terms));
  assert_eq!(ark_bn254_pairing_product(&terms), ArkFq12::ONE);

  let terms = pairing_terms_to_constants(&terms);
  assert_satisfied(&PairingCheckCircuit::new(&terms, true));
}

#[test]
fn pairing_check_three_term_negative_matches_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([70_u8; 32]);
  let q1 = loop {
    let candidate = ArkG2Projective::rand(&mut rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  };
  let q2 = loop {
    let candidate = ArkG2Projective::rand(&mut rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  };
  let p1 = loop {
    let candidate = ArkG1Projective::rand(&mut rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  };
  let p2 = loop {
    let candidate = ArkG1Projective::rand(&mut rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  };
  let p3 = loop {
    let candidate = ArkG1Projective::rand(&mut rng).into_affine();
    if !candidate.is_zero() {
      break candidate;
    }
  };
  let terms = [(p1, q1), (p2, q1), (p3, q2)];
  assert!(!ark_bn254_pairing_check(&terms));

  let terms = pairing_terms_to_constants(&terms);
  assert_satisfied(&PairingCheckCircuit::new(&terms, false));
}

#[test]
fn g2_mixed_add_with_line_same_point_is_not_supported_in_this_slice() {
  let point = ArkG2Affine::generator();
  let current_state = ark_miller_point_from_affine(point);
  let honest_double = (point.into_group() + point).into_affine();
  let (_, honest_line) = ark_double_with_line(current_state);

  assert!(!prover_result(&G2MixedAddWithLineCircuit::new(
    ark_to_miller_point_constant(current_state),
    ark_to_assigned_g2_coords(point),
    ark_to_assigned_g2_coords(honest_double),
    ark_to_line_coeffs_constant(honest_line),
  )));
}

#[test]
fn g2_mixed_add_with_line_inverse_point_is_not_supported_in_this_slice() {
  let point = ArkG2Affine::generator();
  let current_state = ark_miller_point_from_affine(point);
  let unsupported_addend = -point;
  let (_, honest_line) = ark_double_with_line(current_state);

  assert!(!prover_result(&G2MixedAddWithLineCircuit::new(
    ark_to_miller_point_constant(current_state),
    ark_to_assigned_g2_coords(unsupported_addend),
    ark_to_assigned_g2_coords(point),
    ark_to_line_coeffs_constant(honest_line),
  )));
}

#[test]
fn g2_assert_equal_accepts_identical_points() {
  let point = ark_to_assigned_g2_coords(ArkG2Affine::generator());

  assert_satisfied(&G2EqualityCircuit::new(point, point));
}

#[test]
fn g2_assert_equal_rejects_distinct_points() {
  let point = ArkG2Affine::generator();
  let negated = -point;

  assert!(!prover_result(&G2EqualityCircuit::new(
    ark_to_assigned_g2_coords(point),
    ark_to_assigned_g2_coords(negated),
  )));
}

#[test]
fn g2_layout_metrics_are_real_and_nonzero() {
  let on_curve_metrics = g2_on_curve_layout_metrics();
  let neg_metrics = g2_neg_layout_metrics();
  let from_affine_metrics = g2_proj_from_affine_layout_metrics();
  let double_metrics = g2_proj_double_layout_metrics();
  let add_metrics = g2_proj_add_layout_metrics();
  let double_with_line_metrics = g2_double_with_line_layout_metrics();
  let mixed_add_with_line_metrics = g2_mixed_add_with_line_layout_metrics();
  let accumulator_square_metrics = miller_accumulator_square_layout_metrics();
  let accumulator_mul_by_line_metrics = miller_accumulator_mul_by_line_layout_metrics();
  let accumulator_mul_by_line_sparse_metrics =
    miller_accumulator_mul_by_line_sparse_layout_metrics();
  let miller_loop_metrics = miller_loop_layout_metrics();
  let final_exponentiation_metrics = final_exponentiation_layout_metrics();
  let pairing_check_metrics = pairing_check_layout_metrics();

  assert!(on_curve_metrics.rows > 0);
  assert!(neg_metrics.rows > 0);
  assert!(from_affine_metrics.rows > 0);
  assert!(double_metrics.rows > 0);
  assert!(add_metrics.rows > 0);
  assert!(double_with_line_metrics.rows > 0);
  assert!(mixed_add_with_line_metrics.rows > 0);
  assert!(accumulator_square_metrics.rows > 0);
  assert!(accumulator_mul_by_line_metrics.rows > 0);
  assert!(accumulator_mul_by_line_sparse_metrics.rows > 0);
  assert!(miller_loop_metrics.rows > 0);
  assert!(final_exponentiation_metrics.rows > 0);
  assert!(pairing_check_metrics.rows > 0);
  assert!(on_curve_metrics.column_queries > 0);
  assert!(neg_metrics.column_queries > 0);
  assert!(from_affine_metrics.column_queries > 0);
  assert!(double_metrics.column_queries > 0);
  assert!(add_metrics.column_queries > 0);
  assert!(double_with_line_metrics.column_queries > 0);
  assert!(mixed_add_with_line_metrics.column_queries > 0);
  assert!(accumulator_square_metrics.column_queries > 0);
  assert!(accumulator_mul_by_line_metrics.column_queries > 0);
  assert!(accumulator_mul_by_line_sparse_metrics.column_queries > 0);
  assert!(miller_loop_metrics.column_queries > 0);
  assert!(final_exponentiation_metrics.column_queries > 0);
  assert!(pairing_check_metrics.column_queries > 0);
  assert!(accumulator_mul_by_line_sparse_metrics.rows < accumulator_mul_by_line_metrics.rows);
}
