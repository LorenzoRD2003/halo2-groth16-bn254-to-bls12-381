use ff::Field;
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{AssignedFp, Bn254FieldChip, Bn254FieldConfig, ForeignField, NativeField};

/// Assigned BN254 quadratic-extension element represented as `c0 + c1 * u`.
#[derive(Clone, Debug)]
pub struct AssignedFp2 {
  /// Real coefficient in the BN254 quadratic extension.
  pub c0: AssignedFp,
  /// Imaginary coefficient in the BN254 quadratic extension.
  pub c1: AssignedFp,
}

impl AssignedFp2 {
  /// Builds an assigned Fp2 value from its two assigned base-field coordinates.
  #[must_use]
  pub fn new(c0: AssignedFp, c1: AssignedFp) -> Self {
    Self { c0, c1 }
  }

  /// Assigns an Fp2 witness from its two base-field coordinates.
  ///
  /// # Errors
  ///
  /// Returns an error if the underlying foreign-field assignments fail.
  pub fn assign(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
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
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Self::assign(chip, layouter, Value::known(ForeignField::ZERO), Value::known(ForeignField::ZERO))
  }

  /// Assigns the multiplicative identity in Fp2.
  ///
  /// # Errors
  ///
  /// Returns an error if the underlying foreign-field assignments fail.
  pub fn one(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Self::assign(chip, layouter, Value::known(ForeignField::ONE), Value::known(ForeignField::ZERO))
  }

  /// Adds two Fp2 values inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp addition assignment fails.
  pub fn add(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(chip.add(layouter, &self.c0, &rhs.c0)?, chip.add(layouter, &self.c1, &rhs.c1)?))
  }

  /// Subtracts two Fp2 values inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp subtraction assignment fails.
  pub fn sub(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(chip.sub(layouter, &self.c0, &rhs.c0)?, chip.sub(layouter, &self.c1, &rhs.c1)?))
  }

  /// Negates an Fp2 value inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp negation assignment fails.
  pub fn neg(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(chip.neg(layouter, &self.c0)?, chip.neg(layouter, &self.c1)?))
  }

  /// Multiplies two Fp2 values inside the circuit assuming `u^2 = -1`.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp multiplication, addition, or subtraction assignment
  /// fails.
  pub fn mul(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
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
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    let a_sq = chip.square(layouter, &self.c0)?;
    let b_sq = chip.square(layouter, &self.c1)?;
    let ab = chip.mul(layouter, &self.c0, &self.c1)?;
    let two_ab = chip.add(layouter, &ab, &ab)?;

    Ok(Self::new(chip.sub(layouter, &a_sq, &b_sq)?, two_ab))
  }

  /// Asserts coordinate-wise equality against another assigned Fp2 value.
  ///
  /// # Errors
  ///
  /// Returns an error if either coordinate equality constraint cannot be enforced.
  pub fn assert_equal(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<(), Error> {
    chip.assert_equal(layouter, &self.c0, &rhs.c0)?;
    chip.assert_equal(layouter, &self.c1, &rhs.c1)
  }

  /// Asserts coordinate-wise equality against a fixed Fp2 constant.
  ///
  /// # Errors
  ///
  /// Returns an error if either coordinate-equals-constant constraint cannot be enforced.
  pub fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    expected_c0: ForeignField,
    expected_c1: ForeignField,
  ) -> Result<(), Error> {
    chip.assert_equal_to_fixed(layouter, &self.c0, expected_c0)?;
    chip.assert_equal_to_fixed(layouter, &self.c1, expected_c1)
  }
}

/// Small circuit that exercises a single BN254 Fp2 addition.
#[derive(Clone, Debug)]
pub struct Fp2AddCircuit {
  left: (Value<ForeignField>, Value<ForeignField>),
  right: (Value<ForeignField>, Value<ForeignField>),
  expected: (ForeignField, ForeignField),
}

impl Fp2AddCircuit {
  /// Builds a new Fp2 addition circuit with a known expected output.
  #[must_use]
  pub fn new(left: (ForeignField, ForeignField), right: (ForeignField, ForeignField)) -> Self {
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
      left: (Value::unknown(), Value::unknown()),
      right: (Value::unknown(), Value::unknown()),
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254FieldChip::new(&config);
    let left = AssignedFp2::assign(&chip, &mut layouter, self.left.0, self.left.1)?;
    let right = AssignedFp2::assign(&chip, &mut layouter, self.right.0, self.right.1)?;
    let output = left.add(&chip, &mut layouter, &right)?;
    output.assert_equal_to_fixed(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises a single BN254 Fp2 multiplication.
#[derive(Clone, Debug)]
pub struct Fp2MulCircuit {
  left: (Value<ForeignField>, Value<ForeignField>),
  right: (Value<ForeignField>, Value<ForeignField>),
  expected: (ForeignField, ForeignField),
}

impl Fp2MulCircuit {
  /// Builds a new Fp2 multiplication circuit with a known expected output.
  #[must_use]
  pub fn new(left: (ForeignField, ForeignField), right: (ForeignField, ForeignField)) -> Self {
    let ac = left.0 * right.0;
    let bd = left.1 * right.1;
    let ad = left.0 * right.1;
    let bc = left.1 * right.0;

    Self {
      left: (Value::known(left.0), Value::known(left.1)),
      right: (Value::known(right.0), Value::known(right.1)),
      expected: (ac - bd, ad + bc),
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
      left: (Value::unknown(), Value::unknown()),
      right: (Value::unknown(), Value::unknown()),
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254FieldChip::new(&config);
    let left = AssignedFp2::assign(&chip, &mut layouter, self.left.0, self.left.1)?;
    let right = AssignedFp2::assign(&chip, &mut layouter, self.right.0, self.right.1)?;
    let output = left.mul(&chip, &mut layouter, &right)?;
    output.assert_equal_to_fixed(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises a single BN254 Fp2 square.
#[derive(Clone, Debug)]
pub struct Fp2SquareCircuit {
  value: (Value<ForeignField>, Value<ForeignField>),
  expected: (ForeignField, ForeignField),
}

impl Fp2SquareCircuit {
  /// Builds a new Fp2 square circuit with a known expected output.
  #[must_use]
  pub fn new(value: (ForeignField, ForeignField)) -> Self {
    let a_sq = value.0.square();
    let b_sq = value.1.square();
    let ab = value.0 * value.1;

    Self { value: (Value::known(value.0), Value::known(value.1)), expected: (a_sq - b_sq, ab + ab) }
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
    Self { value: (Value::unknown(), Value::unknown()), expected: self.expected }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254FieldChip::new(&config);
    let value = AssignedFp2::assign(&chip, &mut layouter, self.value.0, self.value.1)?;
    let output = value.square(&chip, &mut layouter)?;
    output.assert_equal_to_fixed(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    chip.load(&mut layouter)
  }
}
