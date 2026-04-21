use ff::Field;
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{AssignedFp6, Bn254FieldChip, Bn254FieldConfig, ForeignField, NativeField};

type Fp2Constant = (ForeignField, ForeignField);
type Fp6Value = (
  (Value<ForeignField>, Value<ForeignField>),
  (Value<ForeignField>, Value<ForeignField>),
  (Value<ForeignField>, Value<ForeignField>),
);
type Fp6Constant = (Fp2Constant, Fp2Constant, Fp2Constant);
type Fp12Value = (Fp6Value, Fp6Value);
type Fp12Constant = (Fp6Constant, Fp6Constant);

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

fn fp2_mul_by_fp6_nonresidue_constant(value: Fp2Constant) -> Fp2Constant {
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

fn fp6_sub_constant(left: Fp6Constant, right: Fp6Constant) -> Fp6Constant {
  (
    fp2_sub_constant(left.0, right.0),
    fp2_sub_constant(left.1, right.1),
    fp2_sub_constant(left.2, right.2),
  )
}

fn fp6_mul_by_nonresidue_constant(value: Fp6Constant) -> Fp6Constant {
  (fp2_mul_by_fp6_nonresidue_constant(value.2), value.0, value.1)
}

fn fp6_mul_constant(left: Fp6Constant, right: Fp6Constant) -> Fp6Constant {
  let a_a = fp2_mul_constant(left.0, right.0);
  let b_b = fp2_mul_constant(left.1, right.1);
  let c_c = fp2_mul_constant(left.2, right.2);

  let t1 = fp2_sub_constant(
    fp2_mul_constant(fp2_add_constant(right.1, right.2), fp2_add_constant(left.1, left.2)),
    fp2_add_constant(c_c, b_b),
  );
  let t1 = fp2_add_constant(a_a, fp2_mul_by_fp6_nonresidue_constant(t1));

  let t3 = fp2_sub_constant(
    fp2_mul_constant(fp2_add_constant(right.0, right.2), fp2_add_constant(left.0, left.2)),
    fp2_sub_constant(fp2_add_constant(a_a, c_c), b_b),
  );

  let t2 = fp2_sub_constant(
    fp2_mul_constant(fp2_add_constant(right.0, right.1), fp2_add_constant(left.0, left.1)),
    fp2_add_constant(a_a, b_b),
  );
  let t2 = fp2_add_constant(t2, fp2_mul_by_fp6_nonresidue_constant(c_c));

  (t1, t2, t3)
}

fn fp6_square_constant(value: Fp6Constant) -> Fp6Constant {
  let s0 = fp2_square_constant(value.0);
  let s1 = fp2_add_constant(fp2_mul_constant(value.0, value.1), fp2_mul_constant(value.0, value.1));
  let s2 = fp2_square_constant(fp2_add_constant(fp2_sub_constant(value.0, value.1), value.2));
  let s3 = fp2_add_constant(fp2_mul_constant(value.1, value.2), fp2_mul_constant(value.1, value.2));
  let s4 = fp2_square_constant(value.2);

  (
    fp2_add_constant(fp2_mul_by_fp6_nonresidue_constant(s3), s0),
    fp2_add_constant(fp2_mul_by_fp6_nonresidue_constant(s4), s1),
    fp2_sub_constant(fp2_sub_constant(fp2_add_constant(fp2_add_constant(s1, s2), s3), s0), s4),
  )
}

fn fp12_add_constant(left: &Fp12Constant, right: &Fp12Constant) -> Fp12Constant {
  (fp6_add_constant(left.0, right.0), fp6_add_constant(left.1, right.1))
}

fn fp12_mul_constant(left: &Fp12Constant, right: &Fp12Constant) -> Fp12Constant {
  let a_a = fp6_mul_constant(left.0, right.0);
  let b_b = fp6_mul_constant(left.1, right.1);

  let c0 = fp6_add_constant(a_a, fp6_mul_by_nonresidue_constant(b_b));
  let c1 = fp6_sub_constant(
    fp6_sub_constant(
      fp6_mul_constant(fp6_add_constant(left.0, left.1), fp6_add_constant(right.0, right.1)),
      a_a,
    ),
    b_b,
  );

  (c0, c1)
}

fn fp12_square_constant(value: &Fp12Constant) -> Fp12Constant {
  let a_sq = fp6_square_constant(value.0);
  let b_sq = fp6_square_constant(value.1);
  let ab = fp6_mul_constant(value.0, value.1);

  (fp6_add_constant(a_sq, fp6_mul_by_nonresidue_constant(b_sq)), fp6_add_constant(ab, ab))
}

/// Returns the BN254 Fp12 quadratic nonresidue `w^2 = v` used by arkworks.
///
/// The full tower is
/// `Fp2 = Fp[u] / (u^2 + 1)`,
/// `Fp6 = Fp2[v] / (v^3 - (9 + u))`,
/// `Fp12 = Fp6[w] / (w^2 - v)`,
/// where `v` is represented as `Fp6(0, 1, 0)`.
#[must_use]
pub fn fp12_nonresidue() -> Fp6Constant {
  (
    (ForeignField::ZERO, ForeignField::ZERO),
    (ForeignField::ONE, ForeignField::ZERO),
    (ForeignField::ZERO, ForeignField::ZERO),
  )
}

/// Assigned BN254 Fp12 element represented as `c0 + c1 * w`.
///
/// This follows the arkworks BN254 tower exactly:
/// `Fp2 = Fp[u] / (u^2 + 1)`,
/// `Fp6 = Fp2[v] / (v^3 - (9 + u))`,
/// `Fp12 = Fp6[w] / (w^2 - v)`.
#[derive(Clone, Debug)]
pub struct AssignedFp12 {
  /// Constant coefficient in Fp6.
  pub c0: AssignedFp6,
  /// `w` coefficient in Fp6.
  pub c1: AssignedFp6,
}

impl AssignedFp12 {
  /// Builds an assigned Fp12 value from its two assigned Fp6 coordinates.
  #[must_use]
  pub fn new(c0: AssignedFp6, c1: AssignedFp6) -> Self {
    Self { c0, c1 }
  }

  /// Assigns an Fp12 witness from its two Fp6 coordinates.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp6 assignment fails.
  pub fn assign(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    c0: Fp6Value,
    c1: Fp6Value,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      AssignedFp6::assign(chip, layouter, c0.0, c0.1, c0.2)?,
      AssignedFp6::assign(chip, layouter, c1.0, c1.1, c1.2)?,
    ))
  }

  /// Assigns the additive identity in Fp12.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp6 assignment fails.
  pub fn zero(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(AssignedFp6::zero(chip, layouter)?, AssignedFp6::zero(chip, layouter)?))
  }

  /// Assigns the multiplicative identity in Fp12.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp6 assignment fails.
  pub fn one(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(AssignedFp6::one(chip, layouter)?, AssignedFp6::zero(chip, layouter)?))
  }

  /// Adds two Fp12 values inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp6 addition fails.
  pub fn add(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(self.c0.add(chip, layouter, &rhs.c0)?, self.c1.add(chip, layouter, &rhs.c1)?))
  }

  /// Subtracts two Fp12 values inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp6 subtraction fails.
  pub fn sub(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(self.c0.sub(chip, layouter, &rhs.c0)?, self.c1.sub(chip, layouter, &rhs.c1)?))
  }

  /// Negates an Fp12 value inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp6 negation fails.
  pub fn neg(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(self.c0.neg(chip, layouter)?, self.c1.neg(chip, layouter)?))
  }

  /// Multiplies two Fp12 values inside the circuit using the arkworks-compatible quadratic tower.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp6 operation fails.
  pub fn mul(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    let a_a = self.c0.mul(chip, layouter, &rhs.c0)?;
    let b_b = self.c1.mul(chip, layouter, &rhs.c1)?;
    let b_b_nr = b_b.mul_by_nonresidue(chip, layouter)?;
    let lhs_sum = self.c0.add(chip, layouter, &self.c1)?;
    let rhs_sum = rhs.c0.add(chip, layouter, &rhs.c1)?;
    let cross = lhs_sum.mul(chip, layouter, &rhs_sum)?;

    let c0 = a_a.add(chip, layouter, &b_b_nr)?;
    let c1 = cross.sub(chip, layouter, &a_a)?.sub(chip, layouter, &b_b)?;

    Ok(Self::new(c0, c1))
  }

  /// Squares an Fp12 value inside the circuit using the quadratic-extension identity.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp6 operation fails.
  pub fn square(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    let a_sq = self.c0.square(chip, layouter)?;
    let b_sq = self.c1.square(chip, layouter)?;
    let ab = self.c0.mul(chip, layouter, &self.c1)?;
    let b_sq_nr = b_sq.mul_by_nonresidue(chip, layouter)?;
    let two_ab = ab.add(chip, layouter, &ab)?;

    Ok(Self::new(a_sq.add(chip, layouter, &b_sq_nr)?, two_ab))
  }

  /// Asserts coordinate-wise equality against another assigned Fp12 value.
  ///
  /// # Errors
  ///
  /// Returns an error if any Fp6 coordinate equality constraint fails.
  pub fn assert_equal(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<(), Error> {
    self.c0.assert_equal(chip, layouter, &rhs.c0)?;
    self.c1.assert_equal(chip, layouter, &rhs.c1)
  }

  /// Asserts coordinate-wise equality against a fixed Fp12 constant.
  ///
  /// # Errors
  ///
  /// Returns an error if any Fp6 coordinate-equals-constant constraint fails.
  pub fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    expected: Fp12Constant,
  ) -> Result<(), Error> {
    self.c0.assert_equal_to_fixed(chip, layouter, expected.0)?;
    self.c1.assert_equal_to_fixed(chip, layouter, expected.1)
  }
}

/// Small circuit that exercises a single BN254 Fp12 addition.
#[derive(Clone, Debug)]
pub struct Fp12AddCircuit {
  left: Fp12Value,
  right: Fp12Value,
  expected: Fp12Constant,
}

impl Fp12AddCircuit {
  /// Builds a new Fp12 addition circuit with a known expected output.
  #[must_use]
  pub fn new(left: Fp12Constant, right: Fp12Constant) -> Self {
    Self {
      left: (
        (
          (Value::known(left.0.0.0), Value::known(left.0.0.1)),
          (Value::known(left.0.1.0), Value::known(left.0.1.1)),
          (Value::known(left.0.2.0), Value::known(left.0.2.1)),
        ),
        (
          (Value::known(left.1.0.0), Value::known(left.1.0.1)),
          (Value::known(left.1.1.0), Value::known(left.1.1.1)),
          (Value::known(left.1.2.0), Value::known(left.1.2.1)),
        ),
      ),
      right: (
        (
          (Value::known(right.0.0.0), Value::known(right.0.0.1)),
          (Value::known(right.0.1.0), Value::known(right.0.1.1)),
          (Value::known(right.0.2.0), Value::known(right.0.2.1)),
        ),
        (
          (Value::known(right.1.0.0), Value::known(right.1.0.1)),
          (Value::known(right.1.1.0), Value::known(right.1.1.1)),
          (Value::known(right.1.2.0), Value::known(right.1.2.1)),
        ),
      ),
      expected: fp12_add_constant(&left, &right),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new(
      (
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
      ),
      (
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
      ),
    )
  }
}

impl Default for Fp12AddCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for Fp12AddCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      left: (
        (
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
        ),
        (
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
        ),
      ),
      right: (
        (
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
        ),
        (
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
        ),
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
    let left = AssignedFp12::assign(&chip, &mut layouter, self.left.0, self.left.1)?;
    let right = AssignedFp12::assign(&chip, &mut layouter, self.right.0, self.right.1)?;
    let output = left.add(&chip, &mut layouter, &right)?;
    output.assert_equal_to_fixed(&chip, &mut layouter, self.expected)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises a single BN254 Fp12 multiplication.
#[derive(Clone, Debug)]
pub struct Fp12MulCircuit {
  left: Fp12Value,
  right: Fp12Value,
  expected: Fp12Constant,
}

impl Fp12MulCircuit {
  /// Builds a new Fp12 multiplication circuit with a known expected output.
  #[must_use]
  pub fn new(left: Fp12Constant, right: Fp12Constant) -> Self {
    Self {
      left: (
        (
          (Value::known(left.0.0.0), Value::known(left.0.0.1)),
          (Value::known(left.0.1.0), Value::known(left.0.1.1)),
          (Value::known(left.0.2.0), Value::known(left.0.2.1)),
        ),
        (
          (Value::known(left.1.0.0), Value::known(left.1.0.1)),
          (Value::known(left.1.1.0), Value::known(left.1.1.1)),
          (Value::known(left.1.2.0), Value::known(left.1.2.1)),
        ),
      ),
      right: (
        (
          (Value::known(right.0.0.0), Value::known(right.0.0.1)),
          (Value::known(right.0.1.0), Value::known(right.0.1.1)),
          (Value::known(right.0.2.0), Value::known(right.0.2.1)),
        ),
        (
          (Value::known(right.1.0.0), Value::known(right.1.0.1)),
          (Value::known(right.1.1.0), Value::known(right.1.1.1)),
          (Value::known(right.1.2.0), Value::known(right.1.2.1)),
        ),
      ),
      expected: fp12_mul_constant(&left, &right),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new(
      (
        (
          (ForeignField::from(25_u64), ForeignField::from(26_u64)),
          (ForeignField::from(27_u64), ForeignField::from(28_u64)),
          (ForeignField::from(29_u64), ForeignField::from(30_u64)),
        ),
        (
          (ForeignField::from(31_u64), ForeignField::from(32_u64)),
          (ForeignField::from(33_u64), ForeignField::from(34_u64)),
          (ForeignField::from(35_u64), ForeignField::from(36_u64)),
        ),
      ),
      (
        (
          (ForeignField::from(37_u64), ForeignField::from(38_u64)),
          (ForeignField::from(39_u64), ForeignField::from(40_u64)),
          (ForeignField::from(41_u64), ForeignField::from(42_u64)),
        ),
        (
          (ForeignField::from(43_u64), ForeignField::from(44_u64)),
          (ForeignField::from(45_u64), ForeignField::from(46_u64)),
          (ForeignField::from(47_u64), ForeignField::from(48_u64)),
        ),
      ),
    )
  }
}

impl Default for Fp12MulCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for Fp12MulCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      left: (
        (
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
        ),
        (
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
        ),
      ),
      right: (
        (
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
        ),
        (
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
        ),
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
    let left = AssignedFp12::assign(&chip, &mut layouter, self.left.0, self.left.1)?;
    let right = AssignedFp12::assign(&chip, &mut layouter, self.right.0, self.right.1)?;
    let output = left.mul(&chip, &mut layouter, &right)?;
    output.assert_equal_to_fixed(&chip, &mut layouter, self.expected)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises a single BN254 Fp12 square.
#[derive(Clone, Debug)]
pub struct Fp12SquareCircuit {
  value: Fp12Value,
  expected: Fp12Constant,
}

impl Fp12SquareCircuit {
  /// Builds a new Fp12 square circuit with a known expected output.
  #[must_use]
  pub fn new(value: Fp12Constant) -> Self {
    Self {
      value: (
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
      ),
      expected: fp12_square_constant(&value),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new((
      (
        (ForeignField::from(49_u64), ForeignField::from(50_u64)),
        (ForeignField::from(51_u64), ForeignField::from(52_u64)),
        (ForeignField::from(53_u64), ForeignField::from(54_u64)),
      ),
      (
        (ForeignField::from(55_u64), ForeignField::from(56_u64)),
        (ForeignField::from(57_u64), ForeignField::from(58_u64)),
        (ForeignField::from(59_u64), ForeignField::from(60_u64)),
      ),
    ))
  }
}

impl Default for Fp12SquareCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for Fp12SquareCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      value: (
        (
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
        ),
        (
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
          (Value::unknown(), Value::unknown()),
        ),
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
    let value = AssignedFp12::assign(&chip, &mut layouter, self.value.0, self.value.1)?;
    let output = value.square(&chip, &mut layouter)?;
    output.assert_equal_to_fixed(&chip, &mut layouter, self.expected)?;
    chip.load(&mut layouter)
  }
}
