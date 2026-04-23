use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{
  AssignedCircuitValue, AssignedFieldExt, AssignedFp2, AssignedFp6, Bn254FieldChip,
  Bn254FieldConfig, ForeignField, NativeField,
  host::{
    Fp6Constant, Fp6Value, Fp12Constant, Fp12Value, bn254_final_exponentiation_easy_part_constant,
    fp12_add_constant, fp12_cyclotomic_square_constant, fp12_frobenius_map_constant,
    fp12_inv_constant, fp12_mul_constant, fp12_nonresidue_constant, fp12_square_constant,
  },
  synthesize_binary_value_circuit, synthesize_unary_value_circuit,
};

/// Returns the BN254 Fp12 quadratic nonresidue `w^2 = v` used by arkworks.
///
/// The full tower is
/// `Fp2 = Fp[u] / (u^2 + 1)`,
/// `Fp6 = Fp2[v] / (v^3 - (9 + u))`,
/// `Fp12 = Fp6[w] / (w^2 - v)`,
/// where `v` is represented as `Fp6(0, 1, 0)`.
#[must_use]
pub fn fp12_nonresidue() -> Fp6Constant {
  fp12_nonresidue_constant()
}

/// Assigned BN254 Fp12 element represented as `c0 + c1 * w`.
///
/// This follows the arkworks BN254 tower exactly:
/// `Fp2 = Fp[u] / (u^2 + 1)`,
/// `Fp6 = Fp2[v] / (v^3 - (9 + u))`,
/// `Fp12 = Fp6[w] / (w^2 - v)`.
///
/// This type is the general arithmetic Fp12 layer for the current repository.
/// It intentionally stays field-oriented: pairing-specific sparse multiplication
/// and Miller-loop accumulator semantics live outside this type.
#[derive(Clone, Debug)]
pub struct AssignedFp12 {
  /// Constant coefficient in Fp6.
  pub c0: AssignedFp6,
  /// `w` coefficient in Fp6.
  pub c1: AssignedFp6,
}

fn fp2_value_witness(
  value: Value<(ForeignField, ForeignField)>,
) -> (Value<ForeignField>, Value<ForeignField>) {
  (value.clone().map(|coords| coords.0), value.map(|coords| coords.1))
}

fn fp6_value_witness(value: Value<Fp6Constant>) -> Fp6Value {
  (
    fp2_value_witness(value.clone().map(|coords| coords.0)),
    fp2_value_witness(value.clone().map(|coords| coords.1)),
    fp2_value_witness(value.map(|coords| coords.2)),
  )
}

fn fp12_value_witness(value: Value<Fp12Constant>) -> Fp12Value {
  (
    fp6_value_witness(value.clone().map(|coords| coords.0)),
    fp6_value_witness(value.map(|coords| coords.1)),
  )
}

impl AssignedFp12 {
  fn cyclotomic_square_pair(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    left: &AssignedFp2,
    right: &AssignedFp2,
  ) -> Result<(AssignedFp2, AssignedFp2), Error> {
    // Squares the quadratic element left + right * y where y^2 = (9 + u).
    // This is the Granger-Scott building block reused three times in the full
    // Fp12 cyclotomic square.
    let product = left.mul(chip, layouter, right)?;
    let left_plus_right = left.add(chip, layouter, right)?;
    let right_nr = AssignedFp6::mul_by_nonresidue_fp2(right, chip, layouter)?;
    let left_plus_right_nr = right_nr.add(chip, layouter, left)?;
    let product_nr = AssignedFp6::mul_by_nonresidue_fp2(&product, chip, layouter)?;
    let t0 = left_plus_right
      .mul(chip, layouter, &left_plus_right_nr)?
      .sub(chip, layouter, &product)?
      .sub(chip, layouter, &product_nr)?;
    let t1 = product.add(chip, layouter, &product)?;

    Ok((t0, t1))
  }

  fn fp2_three_t_minus_two_z(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    t: &AssignedFp2,
    z: &AssignedFp2,
  ) -> Result<AssignedFp2, Error> {
    let t_minus_z = t.sub(chip, layouter, z)?;
    let two_t_minus_two_z = t_minus_z.add(chip, layouter, &t_minus_z)?;

    two_t_minus_two_z.add(chip, layouter, t)
  }

  fn fp2_three_t_plus_two_z(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    t: &AssignedFp2,
    z: &AssignedFp2,
  ) -> Result<AssignedFp2, Error> {
    let t_plus_z = t.add(chip, layouter, z)?;
    let two_t_plus_two_z = t_plus_z.add(chip, layouter, &t_plus_z)?;

    two_t_plus_two_z.add(chip, layouter, t)
  }

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
    <Self as AssignedFieldExt>::zero(chip, layouter)
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
    <Self as AssignedFieldExt>::one(chip, layouter)
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
    <Self as AssignedFieldExt>::add(self, chip, layouter, rhs)
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
    <Self as AssignedFieldExt>::sub(self, chip, layouter, rhs)
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
    <Self as AssignedFieldExt>::neg(self, chip, layouter)
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

  /// Squares this Fp12 value under the assumption that it lies in the
  /// cyclotomic subgroup reached after the easy part of BN254 final
  /// exponentiation.
  ///
  /// This implements the Granger-Scott degree-12 cyclotomic formula directly
  /// in the current BN254 tower and must not be used for arbitrary Fp12
  /// elements.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2/Fp6 operation fails.
  pub(crate) fn cyclotomic_square(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    // arkworks / Granger-Scott coefficient order:
    // z0 = c0.c0, z1 = c1.c1, z2 = c1.c0, z3 = c0.c2, z4 = c0.c1, z5 = c1.c2.
    let (t0, t1) = Self::cyclotomic_square_pair(chip, layouter, &self.c0.c0, &self.c1.c1)?;
    let (t2, t3) = Self::cyclotomic_square_pair(chip, layouter, &self.c1.c0, &self.c0.c2)?;
    let (t4, t5) = Self::cyclotomic_square_pair(chip, layouter, &self.c0.c1, &self.c1.c2)?;

    let z0 = Self::fp2_three_t_minus_two_z(chip, layouter, &t0, &self.c0.c0)?;
    let z1 = Self::fp2_three_t_plus_two_z(chip, layouter, &t1, &self.c1.c1)?;
    let t5_nr = AssignedFp6::mul_by_nonresidue_fp2(&t5, chip, layouter)?;
    let z2 = Self::fp2_three_t_plus_two_z(chip, layouter, &t5_nr, &self.c1.c0)?;
    let z3 = Self::fp2_three_t_minus_two_z(chip, layouter, &t4, &self.c0.c2)?;
    let z4 = Self::fp2_three_t_minus_two_z(chip, layouter, &t2, &self.c0.c1)?;
    let z5 = Self::fp2_three_t_plus_two_z(chip, layouter, &t3, &self.c1.c2)?;

    Ok(Self::new(AssignedFp6::new(z0, z4, z3), AssignedFp6::new(z2, z1, z5)))
  }

  pub(crate) fn value(&self) -> Value<Fp12Constant> {
    Value::from_iter([self.c0.value(), self.c1.value()])
      .map(|coords: Vec<Fp6Constant>| (coords[0], coords[1]))
  }

  pub(crate) fn conjugate(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(self.c0.clone(), self.c1.neg(chip, layouter)?))
  }

  pub(crate) fn unitary_inverse(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    self.conjugate(chip, layouter)
  }

  pub(crate) fn inv(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    let inverse = self.value().map(|value| fp12_inv_constant(&value));
    let inverse_witness = fp12_value_witness(inverse);
    let assigned = Self::assign(chip, layouter, inverse_witness.0, inverse_witness.1)?;
    let check = self.mul(chip, layouter, &assigned)?;
    AssignedFp12::one(chip, layouter)?.assert_equal(chip, layouter, &check)?;
    Ok(assigned)
  }

  pub(crate) fn frobenius_map(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    power: usize,
  ) -> Result<Self, Error> {
    let assigned_witness =
      fp12_value_witness(self.value().map(|value| fp12_frobenius_map_constant(&value, power)));
    Self::assign(chip, layouter, assigned_witness.0, assigned_witness.1)
  }

  #[allow(dead_code)]
  pub(crate) fn pow_constant_exp(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    exp: &[u64],
  ) -> Result<Self, Error> {
    let mut result = AssignedFp12::one(chip, layouter)?;
    let mut seen_one = false;

    for limb in exp.iter().rev() {
      for bit in (0..64).rev() {
        if seen_one {
          result = result.square(chip, layouter)?;
        }
        if ((*limb >> bit) & 1) == 1 {
          seen_one = true;
          result = result.mul(chip, layouter, self)?;
        }
      }
    }

    Ok(result)
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
    <Self as AssignedFieldExt>::assert_equal(self, chip, layouter, rhs)
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
    <Self as AssignedFieldExt>::assert_equal_to_fixed(self, chip, layouter, expected)
  }
}

impl AssignedFieldExt for AssignedFp12 {
  type Fixed = Fp12Constant;

  fn zero(chip: &Bn254FieldChip, layouter: &mut impl Layouter<NativeField>) -> Result<Self, Error> {
    Ok(Self::new(AssignedFp6::zero(chip, layouter)?, AssignedFp6::zero(chip, layouter)?))
  }

  fn one(chip: &Bn254FieldChip, layouter: &mut impl Layouter<NativeField>) -> Result<Self, Error> {
    Ok(Self::new(AssignedFp6::one(chip, layouter)?, AssignedFp6::zero(chip, layouter)?))
  }

  fn add(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(self.c0.add(chip, layouter, &rhs.c0)?, self.c1.add(chip, layouter, &rhs.c1)?))
  }

  fn sub(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    Ok(Self::new(self.c0.sub(chip, layouter, &rhs.c0)?, self.c1.sub(chip, layouter, &rhs.c1)?))
  }

  fn neg(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(self.c0.neg(chip, layouter)?, self.c1.neg(chip, layouter)?))
  }

  fn assert_equal(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<(), Error> {
    self.c0.assert_equal(chip, layouter, &rhs.c0)?;
    self.c1.assert_equal(chip, layouter, &rhs.c1)
  }

  fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    expected: Self::Fixed,
  ) -> Result<(), Error> {
    self.c0.assert_equal_to_fixed(chip, layouter, expected.0)?;
    self.c1.assert_equal_to_fixed(chip, layouter, expected.1)
  }
}

impl AssignedCircuitValue for AssignedFp12 {
  type Witness = Fp12Value;

  fn assign_witness(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    witness: Self::Witness,
  ) -> Result<Self, Error> {
    Self::assign(chip, layouter, witness.0, witness.1)
  }

  fn unknown_witness(_witness: &Self::Witness) -> Self::Witness {
    (
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
    )
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
      left: AssignedFp12::unknown_witness(&self.left),
      right: AssignedFp12::unknown_witness(&self.right),
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
    synthesize_binary_value_circuit::<AssignedFp12, _, _>(
      &config,
      layouter,
      self.left,
      self.right,
      self.expected,
      AssignedFp12::add,
    )
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
      left: AssignedFp12::unknown_witness(&self.left),
      right: AssignedFp12::unknown_witness(&self.right),
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
    synthesize_binary_value_circuit::<AssignedFp12, _, _>(
      &config,
      layouter,
      self.left,
      self.right,
      self.expected,
      AssignedFp12::mul,
    )
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

/// Small circuit that exercises a single BN254 Fp12 cyclotomic square.
#[derive(Clone, Debug)]
pub struct Fp12CyclotomicSquareCircuit {
  value: Fp12Value,
  expected: Fp12Constant,
}

impl Fp12CyclotomicSquareCircuit {
  /// Builds a cyclotomic-square circuit from a known cyclotomic-subgroup input.
  ///
  /// The caller is responsible for only providing values that already lie in
  /// the cyclotomic subgroup.
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
      expected: fp12_cyclotomic_square_constant(&value),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let generic_sample = (
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
    );

    Self::new(bn254_final_exponentiation_easy_part_constant(&generic_sample))
  }
}

impl Default for Fp12CyclotomicSquareCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for Fp12CyclotomicSquareCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { value: AssignedFp12::unknown_witness(&self.value), expected: self.expected }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    synthesize_unary_value_circuit::<AssignedFp12, _, _>(
      &config,
      layouter,
      self.value,
      self.expected,
      AssignedFp12::cyclotomic_square,
    )
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
    Self { value: AssignedFp12::unknown_witness(&self.value), expected: self.expected }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    synthesize_unary_value_circuit::<AssignedFp12, _, _>(
      &config,
      layouter,
      self.value,
      self.expected,
      AssignedFp12::square,
    )
  }
}
