use ark_bn254::{
  Fq as ArkFq, Fq2 as ArkFq2, Fq6 as ArkFq6, Fq6Config as ArkFq6Config, Fq12 as ArkFq12,
  Fq12Config as ArkFq12Config, G1Affine as ArkG1Affine, G1Projective as ArkG1Projective,
  G2Affine as ArkG2Affine, G2Projective as ArkG2Projective, g2,
};
use ark_ec::{AffineRepr, CurveGroup, models::short_weierstrass::SWCurveConfig};
use ark_ff::{Field as ArkField, Fp6Config, Fp12Config, UniformRand};
use ff::Field;
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use super::*;
use crate::bn254::g2::{
  MillerAccumulatorMulByLineCircuit, MillerAccumulatorMulByLineSparseCircuit,
};

mod support;
pub(crate) use support::*;
mod accumulator;
mod curve;
mod field_and_tower;
mod pairing;

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
    let initial = assign_fixed_fp12(&chip, &mut layouter, self.initial)?;
    let evaluated_line = assign_fixed_fp12(&chip, &mut layouter, self.evaluated_line)?;
    let expected = assign_fixed_fp12(&chip, &mut layouter, self.expected)?;

    let mut accumulator = AssignedMillerAccumulator::new(initial);
    accumulator.mul_by_evaluated_line(&chip, &mut layouter, &evaluated_line)?;
    accumulator.f.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}
