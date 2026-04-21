use ff::{Field, PrimeField};
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{AssignedFp2, Bn254FieldChip, Bn254FieldConfig, ForeignField, NativeField};

type Fp2Value = (Value<ForeignField>, Value<ForeignField>);
type Fp2Constant = (ForeignField, ForeignField);
type Fp6Value = (Fp2Value, Fp2Value, Fp2Value);
type Fp6Constant = (Fp2Constant, Fp2Constant, Fp2Constant);

/// Returns the BN254 Fp6 cubic nonresidue `v^3 = 9 + u` used by arkworks.
///
/// The tower is `Fp6 = Fp2[v] / (v^3 - (9 + u))`, where `Fp2 = Fp[u] / (u^2 + 1)`.
///
/// # Panics
///
/// Panics if the hard-coded arkworks BN254 Fp6 nonresidue fails to parse.
#[must_use]
pub fn fp6_nonresidue() -> (ForeignField, ForeignField) {
  (
    ForeignField::from_str_vartime("9").expect("hard-coded BN254 Fp6 nonresidue c0 should parse"),
    ForeignField::ONE,
  )
}

fn fp2_add_constant(left: Fp2Constant, right: Fp2Constant) -> Fp2Constant {
  (left.0 + right.0, left.1 + right.1)
}

fn fp2_sub_constant(left: Fp2Constant, right: Fp2Constant) -> Fp2Constant {
  (left.0 - right.0, left.1 - right.1)
}

fn fp2_mul_constant(left: Fp2Constant, right: Fp2Constant) -> Fp2Constant {
  let ac = left.0 * right.0;
  let bd = left.1 * right.1;
  let ad = left.0 * right.1;
  let bc = left.1 * right.0;

  (ac - bd, ad + bc)
}

fn fp2_square_constant(value: Fp2Constant) -> Fp2Constant {
  let a_sq = value.0.square();
  let b_sq = value.1.square();
  let ab = value.0 * value.1;

  (a_sq - b_sq, ab + ab)
}

fn fp2_mul_by_nonresidue_constant(value: Fp2Constant) -> Fp2Constant {
  // (c0 + c1 * u) * (9 + u) = (9*c0 - c1) + (c0 + 9*c1) * u
  let nine_c0 = value.0 * ForeignField::from(9_u64);
  let nine_c1 = value.1 * ForeignField::from(9_u64);
  (nine_c0 - value.1, nine_c1 + value.0)
}

fn fp6_add_constant(left: Fp6Constant, right: Fp6Constant) -> Fp6Constant {
  (
    fp2_add_constant(left.0, right.0),
    fp2_add_constant(left.1, right.1),
    fp2_add_constant(left.2, right.2),
  )
}

fn fp6_mul_constant(left: Fp6Constant, right: Fp6Constant) -> Fp6Constant {
  let a_a = fp2_mul_constant(left.0, right.0);
  let b_b = fp2_mul_constant(left.1, right.1);
  let c_c = fp2_mul_constant(left.2, right.2);

  let t1 = fp2_sub_constant(
    fp2_mul_constant(fp2_add_constant(right.1, right.2), fp2_add_constant(left.1, left.2)),
    fp2_add_constant(c_c, b_b),
  );
  let t1 = fp2_add_constant(a_a, fp2_mul_by_nonresidue_constant(t1));

  let t3 = fp2_sub_constant(
    fp2_mul_constant(fp2_add_constant(right.0, right.2), fp2_add_constant(left.0, left.2)),
    fp2_sub_constant(fp2_add_constant(a_a, c_c), b_b),
  );

  let t2 = fp2_sub_constant(
    fp2_mul_constant(fp2_add_constant(right.0, right.1), fp2_add_constant(left.0, left.1)),
    fp2_add_constant(a_a, b_b),
  );
  let t2 = fp2_add_constant(t2, fp2_mul_by_nonresidue_constant(c_c));

  (t1, t2, t3)
}

fn fp6_square_constant(value: Fp6Constant) -> Fp6Constant {
  let s0 = fp2_square_constant(value.0);
  let s1 = fp2_add_constant(fp2_mul_constant(value.0, value.1), fp2_mul_constant(value.0, value.1));
  let s2 = fp2_square_constant(fp2_add_constant(fp2_sub_constant(value.0, value.1), value.2));
  let s3 = fp2_add_constant(fp2_mul_constant(value.1, value.2), fp2_mul_constant(value.1, value.2));
  let s4 = fp2_square_constant(value.2);

  (
    fp2_add_constant(fp2_mul_by_nonresidue_constant(s3), s0),
    fp2_add_constant(fp2_mul_by_nonresidue_constant(s4), s1),
    fp2_sub_constant(fp2_sub_constant(fp2_add_constant(fp2_add_constant(s1, s2), s3), s0), s4),
  )
}

/// Assigned BN254 Fp6 element represented as `c0 + c1 * v + c2 * v^2`.
///
/// This follows the arkworks BN254 tower exactly:
/// `Fp2 = Fp[u] / (u^2 + 1)` and `Fp6 = Fp2[v] / (v^3 - (9 + u))`.
#[derive(Clone, Debug)]
pub struct AssignedFp6 {
  /// Constant coefficient in Fp2.
  pub c0: AssignedFp2,
  /// `v` coefficient in Fp2.
  pub c1: AssignedFp2,
  /// `v^2` coefficient in Fp2.
  pub c2: AssignedFp2,
}

impl AssignedFp6 {
  /// Builds an assigned Fp6 value from its three assigned Fp2 coordinates.
  #[must_use]
  pub fn new(c0: AssignedFp2, c1: AssignedFp2, c2: AssignedFp2) -> Self {
    Self { c0, c1, c2 }
  }

  /// Assigns an Fp6 witness from its three Fp2 coordinates.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 assignment fails.
  pub fn assign(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
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
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      AssignedFp2::zero(chip, layouter)?,
      AssignedFp2::zero(chip, layouter)?,
      AssignedFp2::zero(chip, layouter)?,
    ))
  }

  /// Assigns the multiplicative identity in Fp6.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 assignment fails.
  pub fn one(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      AssignedFp2::one(chip, layouter)?,
      AssignedFp2::zero(chip, layouter)?,
      AssignedFp2::zero(chip, layouter)?,
    ))
  }

  fn mul_by_nonresidue_fp2(
    value: &AssignedFp2,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<AssignedFp2, Error> {
    // (c0 + c1 * u) * (9 + u) = (9*c0 - c1) + (c0 + 9*c1) * u
    let nine = chip.assign(layouter, Value::known(ForeignField::from(9_u64)))?;
    let nine_c0 = AssignedFp2::new(chip.mul(layouter, &value.c0, &nine)?, value.c1.clone());
    let nine_c1 = AssignedFp2::new(value.c0.clone(), chip.mul(layouter, &value.c1, &nine)?);
    let c0 = chip.sub(layouter, &nine_c0.c0, &value.c1)?;
    let c1 = chip.add(layouter, &value.c0, &nine_c1.c1)?;

    Ok(AssignedFp2::new(c0, c1))
  }

  /// Adds two Fp6 values inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 addition fails.
  pub fn add(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      self.c0.add(chip, layouter, &rhs.c0)?,
      self.c1.add(chip, layouter, &rhs.c1)?,
      self.c2.add(chip, layouter, &rhs.c2)?,
    ))
  }

  /// Subtracts two Fp6 values inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 subtraction fails.
  pub fn sub(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      self.c0.sub(chip, layouter, &rhs.c0)?,
      self.c1.sub(chip, layouter, &rhs.c1)?,
      self.c2.sub(chip, layouter, &rhs.c2)?,
    ))
  }

  /// Negates an Fp6 value inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 negation fails.
  pub fn neg(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      self.c0.neg(chip, layouter)?,
      self.c1.neg(chip, layouter)?,
      self.c2.neg(chip, layouter)?,
    ))
  }

  /// Multiplies two Fp6 values inside the circuit using the arkworks-compatible cubic tower.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 operation fails.
  pub fn mul(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
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
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    let s0 = self.c0.square(chip, layouter)?;
    let s1 = {
      let c0c1 = self.c0.mul(chip, layouter, &self.c1)?;
      c0c1.add(chip, layouter, &c0c1)?
    };
    let s2 = {
      let c0_minus_c1 = self.c0.sub(chip, layouter, &self.c1)?;
      let term = c0_minus_c1.add(chip, layouter, &self.c2)?;
      term.square(chip, layouter)?
    };
    let s3 = {
      let c1c2 = self.c1.mul(chip, layouter, &self.c2)?;
      c1c2.add(chip, layouter, &c1c2)?
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
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<(), Error> {
    self.c0.assert_equal(chip, layouter, &rhs.c0)?;
    self.c1.assert_equal(chip, layouter, &rhs.c1)?;
    self.c2.assert_equal(chip, layouter, &rhs.c2)
  }

  /// Asserts coordinate-wise equality against a fixed Fp6 constant.
  ///
  /// # Errors
  ///
  /// Returns an error if any Fp2 coordinate-equals-constant constraint fails.
  pub fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    expected: Fp6Constant,
  ) -> Result<(), Error> {
    self.c0.assert_equal_to_fixed(chip, layouter, expected.0.0, expected.0.1)?;
    self.c1.assert_equal_to_fixed(chip, layouter, expected.1.0, expected.1.1)?;
    self.c2.assert_equal_to_fixed(chip, layouter, expected.2.0, expected.2.1)
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
      left: (
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
      ),
      right: (
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
      ),
      expected: self.expected,
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
    let left = AssignedFp6::assign(&chip, &mut layouter, self.left.0, self.left.1, self.left.2)?;
    let right =
      AssignedFp6::assign(&chip, &mut layouter, self.right.0, self.right.1, self.right.2)?;
    let output = left.add(&chip, &mut layouter, &right)?;
    output.assert_equal_to_fixed(&chip, &mut layouter, self.expected)?;
    chip.load(&mut layouter)
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
      left: (
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
      ),
      right: (
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
      ),
      expected: self.expected,
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
    let left = AssignedFp6::assign(&chip, &mut layouter, self.left.0, self.left.1, self.left.2)?;
    let right =
      AssignedFp6::assign(&chip, &mut layouter, self.right.0, self.right.1, self.right.2)?;
    let output = left.mul(&chip, &mut layouter, &right)?;
    output.assert_equal_to_fixed(&chip, &mut layouter, self.expected)?;
    chip.load(&mut layouter)
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
      value: (
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
      ),
      expected: self.expected,
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
    let value =
      AssignedFp6::assign(&chip, &mut layouter, self.value.0, self.value.1, self.value.2)?;
    let output = value.square(&chip, &mut layouter)?;
    output.assert_equal_to_fixed(&chip, &mut layouter, self.expected)?;
    chip.load(&mut layouter)
  }
}
