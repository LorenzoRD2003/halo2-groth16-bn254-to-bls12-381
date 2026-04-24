use ff::{Field, PrimeField};
use midnight_circuits::field::foreign::params::{FieldEmulationParams, MultiEmulationParams};
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{
  AssignedCircuitValue, AssignedFieldExt, AssignedFp2, Bn254FieldChip, Bn254FieldConfig,
  ForeignField, NativeField,
  host::{
    Fp2Constant, Fp2Value, Fp6Constant, Fp6Value, fp6_add_constant, fp6_mul_constant,
    fp6_nonresidue_constant, fp6_square_constant,
  },
  synthesize_binary_value_circuit, synthesize_unary_value_circuit,
};

/// Returns the BN254 Fp6 cubic nonresidue `v^3 = 9 + u` used by arkworks.
///
/// The tower is `Fp6 = Fp2[v] / (v^3 - (9 + u))`, where `Fp2 = Fp[u] / (u^2 + 1)`.
///
/// # Panics
///
/// Panics if the hard-coded arkworks BN254 Fp6 nonresidue fails to parse.
#[must_use]
pub fn fp6_nonresidue() -> (ForeignField, ForeignField) {
  fp6_nonresidue_constant()
}

/// Assigned BN254 Fp6 element represented as `c0 + c1 * v + c2 * v^2`.
///
/// This follows the arkworks BN254 tower exactly:
/// `Fp2 = Fp[u] / (u^2 + 1)` and `Fp6 = Fp2[v] / (v^3 - (9 + u))`.
#[derive(Clone, Debug)]
pub struct AssignedFp6<FHost = NativeField>
where
  FHost: PrimeField,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  /// Constant coefficient in Fp2.
  pub c0: AssignedFp2<FHost>,
  /// `v` coefficient in Fp2.
  pub c1: AssignedFp2<FHost>,
  /// `v^2` coefficient in Fp2.
  pub c2: AssignedFp2<FHost>,
}

impl<FHost> AssignedFp6<FHost>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  /// Builds an assigned Fp6 value from its three assigned Fp2 coordinates.
  #[must_use]
  pub fn new(c0: AssignedFp2<FHost>, c1: AssignedFp2<FHost>, c2: AssignedFp2<FHost>) -> Self {
    Self { c0, c1, c2 }
  }

  /// Assigns an Fp6 witness from its three Fp2 coordinates.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 assignment fails.
  pub fn assign(
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    c0: Fp2Value,
    c1: Fp2Value,
    c2: Fp2Value,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      AssignedFp2::assign(chip, layouter, c0.0, c0.1)?,
      AssignedFp2::assign(chip, layouter, c1.0, c1.1)?,
      AssignedFp2::assign(chip, layouter, c2.0, c2.1)?,
    ))
  }

  /// Assigns the additive identity in Fp6.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 assignment fails.
  pub fn zero(
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    <Self as AssignedFieldExt<FHost>>::zero(chip, layouter)
  }

  /// Assigns the multiplicative identity in Fp6.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 assignment fails.
  pub fn one(
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    <Self as AssignedFieldExt<FHost>>::one(chip, layouter)
  }

  pub(crate) fn mul_by_nonresidue_fp2(
    value: &AssignedFp2<FHost>,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<AssignedFp2<FHost>, Error> {
    // (c0 + c1 * u) * (9 + u) = (9*c0 - c1) + (c0 + 9*c1) * u
    let nine_c0 = AssignedFp2::new(
      chip.mul_by_constant(layouter, &value.c0, ForeignField::from(9_u64))?,
      value.c1.clone(),
    );
    let nine_c1 = AssignedFp2::new(
      value.c0.clone(),
      chip.mul_by_constant(layouter, &value.c1, ForeignField::from(9_u64))?,
    );
    let c0 = chip.sub(layouter, &nine_c0.c0, &value.c1)?;
    let c1 = chip.add(layouter, &value.c0, &nine_c1.c1)?;

    Ok(AssignedFp2::new(c0, c1))
  }

  pub(crate) fn scale_by_fp2(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    scalar: &AssignedFp2<FHost>,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      self.c0.mul(chip, layouter, scalar)?,
      self.c1.mul(chip, layouter, scalar)?,
      self.c2.mul(chip, layouter, scalar)?,
    ))
  }

  pub(crate) fn scale_by_base_constant(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    scalar: ForeignField,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      self.c0.scale_by_constant(chip, layouter, scalar)?,
      self.c1.scale_by_constant(chip, layouter, scalar)?,
      self.c2.scale_by_constant(chip, layouter, scalar)?,
    ))
  }

  pub(crate) fn value(&self) -> Value<Fp6Constant> {
    Value::from_iter([self.c0.value(), self.c1.value(), self.c2.value()])
      .map(|coords: Vec<Fp2Constant>| (coords[0], coords[1], coords[2]))
  }

  /// Multiplies this Fp6 value by a sparse `Fp6(c0, c1, 0)`.
  ///
  /// This is the exact cubic-tower specialization used by later `mul_by_034`
  /// style Fp12 products. Keeping it crate-private avoids broadening the
  /// extension-field public API with pairing-specific helpers.
  pub(crate) fn mul_by_01(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    c0: &AssignedFp2<FHost>,
    c1: &AssignedFp2<FHost>,
  ) -> Result<Self, Error> {
    let z0_x0 = self.c0.mul(chip, layouter, c0)?;
    let z1_x1 = self.c1.mul(chip, layouter, c1)?;
    let z2_x1 = self.c2.mul(chip, layouter, c1)?;
    let z2_x1_nr = Self::mul_by_nonresidue_fp2(&z2_x1, chip, layouter)?;
    let out_c0 = z0_x0.add(chip, layouter, &z2_x1_nr)?;
    let z0_x1 = self.c0.mul(chip, layouter, c1)?;
    let z1_x0 = self.c1.mul(chip, layouter, c0)?;
    let out_c1 = z0_x1.add(chip, layouter, &z1_x0)?;
    let out_c2 = self.c2.mul(chip, layouter, c0)?.add(chip, layouter, &z1_x1)?;

    Ok(Self::new(out_c0, out_c1, out_c2))
  }

  /// Multiplies an Fp6 value by the arkworks BN254 quadratic-over-cubic nonresidue `v`.
  ///
  /// The Fp12 tower is `Fp12 = Fp6[w] / (w^2 - v)`, so multiplication by `v`
  /// maps `(c0, c1, c2)` to `(c2 * (9 + u), c0, c1)` inside the existing Fp6 tower.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 operation fails.
  pub fn mul_by_nonresidue(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      Self::mul_by_nonresidue_fp2(&self.c2, chip, layouter)?,
      self.c0.clone(),
      self.c1.clone(),
    ))
  }

  /// Adds two Fp6 values inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 addition fails.
  pub fn add(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    <Self as AssignedFieldExt<FHost>>::add(self, chip, layouter, rhs)
  }

  /// Subtracts two Fp6 values inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 subtraction fails.
  pub fn sub(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    <Self as AssignedFieldExt<FHost>>::sub(self, chip, layouter, rhs)
  }

  /// Negates an Fp6 value inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 negation fails.
  pub fn neg(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    <Self as AssignedFieldExt<FHost>>::neg(self, chip, layouter)
  }

  /// Multiplies two Fp6 values inside the circuit using the arkworks-compatible cubic tower.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 operation fails.
  pub fn mul(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    let a_a = self.c0.mul(chip, layouter, &rhs.c0)?;
    let b_b = self.c1.mul(chip, layouter, &rhs.c1)?;
    let c_c = self.c2.mul(chip, layouter, &rhs.c2)?;

    let t1 = {
      let rhs_c1_plus_c2 = rhs.c1.add(chip, layouter, &rhs.c2)?;
      let lhs_c1_plus_c2 = self.c1.add(chip, layouter, &self.c2)?;
      let cross = rhs_c1_plus_c2.mul(chip, layouter, &lhs_c1_plus_c2)?;
      let c_c_plus_b_b = c_c.add(chip, layouter, &b_b)?;
      let cross = cross.sub(chip, layouter, &c_c_plus_b_b)?;
      let cross = Self::mul_by_nonresidue_fp2(&cross, chip, layouter)?;
      a_a.add(chip, layouter, &cross)?
    };

    let t3 = {
      let rhs_c0_plus_c2 = rhs.c0.add(chip, layouter, &rhs.c2)?;
      let lhs_c0_plus_c2 = self.c0.add(chip, layouter, &self.c2)?;
      let cross = rhs_c0_plus_c2.mul(chip, layouter, &lhs_c0_plus_c2)?;
      let a_a_plus_c_c = a_a.add(chip, layouter, &c_c)?;
      let to_subtract = a_a_plus_c_c.sub(chip, layouter, &b_b)?;
      cross.sub(chip, layouter, &to_subtract)?
    };

    let t2 = {
      let rhs_c0_plus_c1 = rhs.c0.add(chip, layouter, &rhs.c1)?;
      let lhs_c0_plus_c1 = self.c0.add(chip, layouter, &self.c1)?;
      let cross = rhs_c0_plus_c1.mul(chip, layouter, &lhs_c0_plus_c1)?;
      let a_a_plus_b_b = a_a.add(chip, layouter, &b_b)?;
      let cross = cross.sub(chip, layouter, &a_a_plus_b_b)?;
      let c_c_nr = Self::mul_by_nonresidue_fp2(&c_c, chip, layouter)?;
      cross.add(chip, layouter, &c_c_nr)?
    };

    Ok(Self::new(t1, t2, t3))
  }

  /// Squares an Fp6 value inside the circuit using the standard cubic-extension square formula.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 operation fails.
  pub fn square(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    let s0 = self.c0.square(chip, layouter)?;
    let s1 = {
      let c0c1 = self.c0.mul(chip, layouter, &self.c1)?;
      c0c1.scale_by_constant(chip, layouter, ForeignField::from(2_u64))?
    };
    let s2 = {
      let c0_minus_c1 = self.c0.sub(chip, layouter, &self.c1)?;
      let term = c0_minus_c1.add(chip, layouter, &self.c2)?;
      term.square(chip, layouter)?
    };
    let s3 = {
      let c1c2 = self.c1.mul(chip, layouter, &self.c2)?;
      c1c2.scale_by_constant(chip, layouter, ForeignField::from(2_u64))?
    };
    let s4 = self.c2.square(chip, layouter)?;

    let c0 = {
      let s3_nr = Self::mul_by_nonresidue_fp2(&s3, chip, layouter)?;
      s3_nr.add(chip, layouter, &s0)?
    };
    let c1 = {
      let s4_nr = Self::mul_by_nonresidue_fp2(&s4, chip, layouter)?;
      s4_nr.add(chip, layouter, &s1)?
    };
    let c2 = {
      let sum = s1.add(chip, layouter, &s2)?.add(chip, layouter, &s3)?;
      sum.sub(chip, layouter, &s0)?.sub(chip, layouter, &s4)?
    };

    Ok(Self::new(c0, c1, c2))
  }

  /// Asserts coordinate-wise equality against another assigned Fp6 value.
  ///
  /// # Errors
  ///
  /// Returns an error if any Fp2 coordinate equality constraint fails.
  pub fn assert_equal(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<(), Error> {
    <Self as AssignedFieldExt<FHost>>::assert_equal(self, chip, layouter, rhs)
  }

  /// Asserts coordinate-wise equality against a fixed Fp6 constant.
  ///
  /// # Errors
  ///
  /// Returns an error if any Fp2 coordinate-equals-constant constraint fails.
  pub fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    expected: Fp6Constant,
  ) -> Result<(), Error> {
    <Self as AssignedFieldExt<FHost>>::assert_equal_to_fixed(self, chip, layouter, expected)
  }
}

impl<FHost> AssignedFieldExt<FHost> for AssignedFp6<FHost>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  type Fixed = Fp6Constant;

  fn zero(
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      AssignedFp2::zero(chip, layouter)?,
      AssignedFp2::zero(chip, layouter)?,
      AssignedFp2::zero(chip, layouter)?,
    ))
  }

  fn one(chip: &Bn254FieldChip<FHost>, layouter: &mut impl Layouter<FHost>) -> Result<Self, Error> {
    Ok(Self::new(
      AssignedFp2::one(chip, layouter)?,
      AssignedFp2::zero(chip, layouter)?,
      AssignedFp2::zero(chip, layouter)?,
    ))
  }

  fn add(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      self.c0.add(chip, layouter, &rhs.c0)?,
      self.c1.add(chip, layouter, &rhs.c1)?,
      self.c2.add(chip, layouter, &rhs.c2)?,
    ))
  }

  fn sub(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      self.c0.sub(chip, layouter, &rhs.c0)?,
      self.c1.sub(chip, layouter, &rhs.c1)?,
      self.c2.sub(chip, layouter, &rhs.c2)?,
    ))
  }

  fn neg(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      self.c0.neg(chip, layouter)?,
      self.c1.neg(chip, layouter)?,
      self.c2.neg(chip, layouter)?,
    ))
  }

  fn assert_equal(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<(), Error> {
    self.c0.assert_equal(chip, layouter, &rhs.c0)?;
    self.c1.assert_equal(chip, layouter, &rhs.c1)?;
    self.c2.assert_equal(chip, layouter, &rhs.c2)
  }

  fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    expected: Self::Fixed,
  ) -> Result<(), Error> {
    self.c0.assert_equal_to_fixed(chip, layouter, expected.0.0, expected.0.1)?;
    self.c1.assert_equal_to_fixed(chip, layouter, expected.1.0, expected.1.1)?;
    self.c2.assert_equal_to_fixed(chip, layouter, expected.2.0, expected.2.1)
  }
}

impl<FHost> AssignedCircuitValue<FHost> for AssignedFp6<FHost>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  type Witness = Fp6Value;

  fn assign_witness(
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    witness: Self::Witness,
  ) -> Result<Self, Error> {
    Self::assign(chip, layouter, witness.0, witness.1, witness.2)
  }

  fn unknown_witness(_witness: &Self::Witness) -> Self::Witness {
    (
      (Value::unknown(), Value::unknown()),
      (Value::unknown(), Value::unknown()),
      (Value::unknown(), Value::unknown()),
    )
  }
}

/// Small circuit that exercises a single BN254 Fp6 addition.
#[derive(Clone, Debug)]
pub struct Fp6AddCircuit {
  left: Fp6Value,
  right: Fp6Value,
  expected: Fp6Constant,
}

impl Fp6AddCircuit {
  /// Builds a new Fp6 addition circuit with a known expected output.
  #[must_use]
  pub fn new(left: Fp6Constant, right: Fp6Constant) -> Self {
    Self {
      left: (
        (Value::known(left.0.0), Value::known(left.0.1)),
        (Value::known(left.1.0), Value::known(left.1.1)),
        (Value::known(left.2.0), Value::known(left.2.1)),
      ),
      right: (
        (Value::known(right.0.0), Value::known(right.0.1)),
        (Value::known(right.1.0), Value::known(right.1.1)),
        (Value::known(right.2.0), Value::known(right.2.1)),
      ),
      expected: fp6_add_constant(left, right),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new(
      (
        (ForeignField::from(1_u64), ForeignField::from(2_u64)),
        (ForeignField::from(3_u64), ForeignField::from(4_u64)),
        (ForeignField::from(5_u64), ForeignField::from(6_u64)),
      ),
      (
        (ForeignField::from(7_u64), ForeignField::from(8_u64)),
        (ForeignField::from(9_u64), ForeignField::from(10_u64)),
        (ForeignField::from(11_u64), ForeignField::from(12_u64)),
      ),
    )
  }
}

impl Default for Fp6AddCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for Fp6AddCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      left: AssignedFp6::<NativeField>::unknown_witness(&self.left),
      right: AssignedFp6::<NativeField>::unknown_witness(&self.right),
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    synthesize_binary_value_circuit::<AssignedFp6, _, _>(
      &config,
      layouter,
      self.left,
      self.right,
      self.expected,
      AssignedFp6::add,
    )
  }
}

/// Small circuit that exercises a single BN254 Fp6 multiplication.
#[derive(Clone, Debug)]
pub struct Fp6MulCircuit {
  left: Fp6Value,
  right: Fp6Value,
  expected: Fp6Constant,
}

impl Fp6MulCircuit {
  /// Builds a new Fp6 multiplication circuit with a known expected output.
  #[must_use]
  pub fn new(left: Fp6Constant, right: Fp6Constant) -> Self {
    Self {
      left: (
        (Value::known(left.0.0), Value::known(left.0.1)),
        (Value::known(left.1.0), Value::known(left.1.1)),
        (Value::known(left.2.0), Value::known(left.2.1)),
      ),
      right: (
        (Value::known(right.0.0), Value::known(right.0.1)),
        (Value::known(right.1.0), Value::known(right.1.1)),
        (Value::known(right.2.0), Value::known(right.2.1)),
      ),
      expected: fp6_mul_constant(left, right),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new(
      (
        (ForeignField::from(13_u64), ForeignField::from(14_u64)),
        (ForeignField::from(15_u64), ForeignField::from(16_u64)),
        (ForeignField::from(17_u64), ForeignField::from(18_u64)),
      ),
      (
        (ForeignField::from(19_u64), ForeignField::from(20_u64)),
        (ForeignField::from(21_u64), ForeignField::from(22_u64)),
        (ForeignField::from(23_u64), ForeignField::from(24_u64)),
      ),
    )
  }
}

impl Default for Fp6MulCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for Fp6MulCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      left: AssignedFp6::<NativeField>::unknown_witness(&self.left),
      right: AssignedFp6::<NativeField>::unknown_witness(&self.right),
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    synthesize_binary_value_circuit::<AssignedFp6, _, _>(
      &config,
      layouter,
      self.left,
      self.right,
      self.expected,
      AssignedFp6::mul,
    )
  }
}

/// Small circuit that exercises a single BN254 Fp6 square.
#[derive(Clone, Debug)]
pub struct Fp6SquareCircuit {
  value: Fp6Value,
  expected: Fp6Constant,
}

impl Fp6SquareCircuit {
  /// Builds a new Fp6 square circuit with a known expected output.
  #[must_use]
  pub fn new(value: Fp6Constant) -> Self {
    Self {
      value: (
        (Value::known(value.0.0), Value::known(value.0.1)),
        (Value::known(value.1.0), Value::known(value.1.1)),
        (Value::known(value.2.0), Value::known(value.2.1)),
      ),
      expected: fp6_square_constant(value),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new((
      (ForeignField::from(25_u64), ForeignField::from(26_u64)),
      (ForeignField::from(27_u64), ForeignField::from(28_u64)),
      (ForeignField::from(29_u64), ForeignField::from(30_u64)),
    ))
  }
}

impl Default for Fp6SquareCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for Fp6SquareCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      value: AssignedFp6::<NativeField>::unknown_witness(&self.value),
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    synthesize_unary_value_circuit::<AssignedFp6, _, _>(
      &config,
      layouter,
      self.value,
      self.expected,
      AssignedFp6::square,
    )
  }
}
