use ff::{Field, PrimeField};
use midnight_circuits::field::foreign::params::{FieldEmulationParams, MultiEmulationParams};
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};
use midnight_circuits::types::InnerValue;

use super::{
  AssignedCircuitValue, AssignedFieldExt, AssignedFp, Bn254FieldChip, Bn254FieldConfig,
  ForeignField, NativeField,
  host::{Fp2Constant, Fp2Value, fp2_mul_constant, fp2_square_constant},
  synthesize_binary_value_circuit, synthesize_unary_value_circuit,
};

/// Assigned BN254 quadratic-extension element represented as `c0 + c1 * u`.
#[derive(Clone, Debug)]
pub struct AssignedFp2<FHost = NativeField>
where
  FHost: PrimeField,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  /// Real coefficient in the BN254 quadratic extension.
  pub c0: AssignedFp<FHost>,
  /// Imaginary coefficient in the BN254 quadratic extension.
  pub c1: AssignedFp<FHost>,
}

impl<FHost> AssignedFp2<FHost>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  /// Builds an assigned Fp2 value from its two assigned base-field coordinates.
  #[must_use]
  pub fn new(c0: AssignedFp<FHost>, c1: AssignedFp<FHost>) -> Self {
    Self { c0, c1 }
  }

  /// Assigns an Fp2 witness from its two base-field coordinates.
  ///
  /// # Errors
  ///
  /// Returns an error if the underlying foreign-field assignments fail.
  pub fn assign(
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    c0: Value<ForeignField>,
    c1: Value<ForeignField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(chip.assign(layouter, c0)?, chip.assign(layouter, c1)?))
  }

  /// Assigns the additive identity in Fp2.
  ///
  /// # Errors
  ///
  /// Returns an error if the underlying foreign-field assignments fail.
  pub fn zero(
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    <Self as AssignedFieldExt<FHost>>::zero(chip, layouter)
  }

  /// Assigns the multiplicative identity in Fp2.
  ///
  /// # Errors
  ///
  /// Returns an error if the underlying foreign-field assignments fail.
  pub fn one(
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    <Self as AssignedFieldExt<FHost>>::one(chip, layouter)
  }

  /// Adds two Fp2 values inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp addition assignment fails.
  pub fn add(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    <Self as AssignedFieldExt<FHost>>::add(self, chip, layouter, rhs)
  }

  /// Subtracts two Fp2 values inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp subtraction assignment fails.
  pub fn sub(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    <Self as AssignedFieldExt<FHost>>::sub(self, chip, layouter, rhs)
  }

  /// Negates an Fp2 value inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp negation assignment fails.
  pub fn neg(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    <Self as AssignedFieldExt<FHost>>::neg(self, chip, layouter)
  }

  /// Multiplies two Fp2 values inside the circuit assuming `u^2 = -1`.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp multiplication, addition, or subtraction assignment
  /// fails.
  pub fn mul(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    let ac = chip.mul(layouter, &self.c0, &rhs.c0)?;
    let bd = chip.mul(layouter, &self.c1, &rhs.c1)?;
    let ad = chip.mul(layouter, &self.c0, &rhs.c1)?;
    let bc = chip.mul(layouter, &self.c1, &rhs.c0)?;

    Ok(Self::new(chip.sub(layouter, &ac, &bd)?, chip.add(layouter, &ad, &bc)?))
  }

  /// Squares an Fp2 value inside the circuit assuming `u^2 = -1`.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp multiplication, addition, or subtraction assignment
  /// fails.
  pub fn square(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    let a_sq = chip.square(layouter, &self.c0)?;
    let b_sq = chip.square(layouter, &self.c1)?;
    let ab = chip.mul(layouter, &self.c0, &self.c1)?;
    let two_ab = chip.mul_by_constant(layouter, &ab, ForeignField::from(2_u64))?;

    Ok(Self::new(chip.sub(layouter, &a_sq, &b_sq)?, two_ab))
  }

  /// Scales an Fp2 value by a BN254 base-field element inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if either coordinate multiplication fails.
  pub fn scale_by_fp(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    scalar: &AssignedFp<FHost>,
  ) -> Result<Self, Error> {
    Ok(Self::new(chip.mul(layouter, &self.c0, scalar)?, chip.mul(layouter, &self.c1, scalar)?))
  }

  pub(crate) fn scale_by_constant(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    scalar: ForeignField,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      chip.mul_by_constant(layouter, &self.c0, scalar)?,
      chip.mul_by_constant(layouter, &self.c1, scalar)?,
    ))
  }

  pub(crate) fn mul_by_constant(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: Fp2Constant,
  ) -> Result<Self, Error> {
    let ac = chip.mul_by_constant(layouter, &self.c0, rhs.0)?;
    let bd = chip.mul_by_constant(layouter, &self.c1, rhs.1)?;
    let ad = chip.mul_by_constant(layouter, &self.c0, rhs.1)?;
    let bc = chip.mul_by_constant(layouter, &self.c1, rhs.0)?;

    Ok(Self::new(chip.sub(layouter, &ac, &bd)?, chip.add(layouter, &ad, &bc)?))
  }

  pub(crate) fn value(&self) -> Value<Fp2Constant> {
    Value::from_iter([self.c0.value(), self.c1.value()])
      .map(|coords: Vec<ForeignField>| (coords[0], coords[1]))
  }

  /// Asserts coordinate-wise equality against another assigned Fp2 value.
  ///
  /// # Errors
  ///
  /// Returns an error if either coordinate equality constraint cannot be enforced.
  pub fn assert_equal(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<(), Error> {
    <Self as AssignedFieldExt<FHost>>::assert_equal(self, chip, layouter, rhs)
  }

  /// Asserts coordinate-wise equality against a fixed Fp2 constant.
  ///
  /// # Errors
  ///
  /// Returns an error if either coordinate-equals-constant constraint cannot be enforced.
  pub fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    expected_c0: ForeignField,
    expected_c1: ForeignField,
  ) -> Result<(), Error> {
    <Self as AssignedFieldExt<FHost>>::assert_equal_to_fixed(
      self,
      chip,
      layouter,
      (expected_c0, expected_c1),
    )
  }
}

impl<FHost> AssignedFieldExt<FHost> for AssignedFp2<FHost>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  type Fixed = (ForeignField, ForeignField);

  fn zero(
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    Self::assign(chip, layouter, Value::known(ForeignField::ZERO), Value::known(ForeignField::ZERO))
  }

  fn one(chip: &Bn254FieldChip<FHost>, layouter: &mut impl Layouter<FHost>) -> Result<Self, Error> {
    Self::assign(chip, layouter, Value::known(ForeignField::ONE), Value::known(ForeignField::ZERO))
  }

  fn add(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(chip.add(layouter, &self.c0, &rhs.c0)?, chip.add(layouter, &self.c1, &rhs.c1)?))
  }

  fn sub(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(chip.sub(layouter, &self.c0, &rhs.c0)?, chip.sub(layouter, &self.c1, &rhs.c1)?))
  }

  fn neg(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    Ok(Self::new(chip.neg(layouter, &self.c0)?, chip.neg(layouter, &self.c1)?))
  }

  fn assert_equal(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<(), Error> {
    chip.assert_equal(layouter, &self.c0, &rhs.c0)?;
    chip.assert_equal(layouter, &self.c1, &rhs.c1)
  }

  fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    expected: Self::Fixed,
  ) -> Result<(), Error> {
    chip.assert_equal_to_fixed(layouter, &self.c0, expected.0)?;
    chip.assert_equal_to_fixed(layouter, &self.c1, expected.1)
  }
}

impl<FHost> AssignedCircuitValue<FHost> for AssignedFp2<FHost>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  type Witness = (Value<ForeignField>, Value<ForeignField>);

  fn assign_witness(
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    witness: Self::Witness,
  ) -> Result<Self, Error> {
    Self::assign(chip, layouter, witness.0, witness.1)
  }

  fn unknown_witness(_witness: &Self::Witness) -> Self::Witness {
    (Value::unknown(), Value::unknown())
  }
}

/// Small circuit that exercises a single BN254 Fp2 addition.
#[derive(Clone, Debug)]
pub struct Fp2AddCircuit {
  left: Fp2Value,
  right: Fp2Value,
  expected: Fp2Constant,
}

impl Fp2AddCircuit {
  /// Builds a new Fp2 addition circuit with a known expected output.
  #[must_use]
  pub fn new(left: Fp2Constant, right: Fp2Constant) -> Self {
    Self {
      left: (Value::known(left.0), Value::known(left.1)),
      right: (Value::known(right.0), Value::known(right.1)),
      expected: (left.0 + right.0, left.1 + right.1),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new(
      (ForeignField::from(3), ForeignField::from(5)),
      (ForeignField::from(7), ForeignField::from(11)),
    )
  }
}

impl Default for Fp2AddCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for Fp2AddCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      left: AssignedFp2::<NativeField>::unknown_witness(&self.left),
      right: AssignedFp2::<NativeField>::unknown_witness(&self.right),
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    synthesize_binary_value_circuit::<AssignedFp2, _, _>(
      &config,
      layouter,
      self.left,
      self.right,
      self.expected,
      AssignedFp2::add,
    )
  }
}

/// Small circuit that exercises a single BN254 Fp2 multiplication.
#[derive(Clone, Debug)]
pub struct Fp2MulCircuit {
  left: Fp2Value,
  right: Fp2Value,
  expected: Fp2Constant,
}

impl Fp2MulCircuit {
  /// Builds a new Fp2 multiplication circuit with a known expected output.
  #[must_use]
  pub fn new(left: Fp2Constant, right: Fp2Constant) -> Self {
    Self {
      left: (Value::known(left.0), Value::known(left.1)),
      right: (Value::known(right.0), Value::known(right.1)),
      expected: fp2_mul_constant(left, right),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new(
      (ForeignField::from(13), ForeignField::from(17)),
      (ForeignField::from(19), ForeignField::from(23)),
    )
  }
}

impl Default for Fp2MulCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for Fp2MulCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      left: AssignedFp2::<NativeField>::unknown_witness(&self.left),
      right: AssignedFp2::<NativeField>::unknown_witness(&self.right),
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    synthesize_binary_value_circuit::<AssignedFp2, _, _>(
      &config,
      layouter,
      self.left,
      self.right,
      self.expected,
      AssignedFp2::mul,
    )
  }
}

/// Small circuit that exercises a single BN254 Fp2 square.
#[derive(Clone, Debug)]
pub struct Fp2SquareCircuit {
  value: Fp2Value,
  expected: Fp2Constant,
}

impl Fp2SquareCircuit {
  /// Builds a new Fp2 square circuit with a known expected output.
  #[must_use]
  pub fn new(value: Fp2Constant) -> Self {
    Self {
      value: (Value::known(value.0), Value::known(value.1)),
      expected: fp2_square_constant(value),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new((ForeignField::from(29), ForeignField::from(31)))
  }
}

impl Default for Fp2SquareCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for Fp2SquareCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      value: AssignedFp2::<NativeField>::unknown_witness(&self.value),
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    synthesize_unary_value_circuit::<AssignedFp2, _, _>(
      &config,
      layouter,
      self.value,
      self.expected,
      AssignedFp2::square,
    )
  }
}
