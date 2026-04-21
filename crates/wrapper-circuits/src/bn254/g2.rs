use ff::{Field, PrimeField};
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{AssignedFp2, Bn254FieldChip, Bn254FieldConfig, ForeignField, NativeField};

type Fp2Value = (Value<ForeignField>, Value<ForeignField>);
type Fp2Constant = (ForeignField, ForeignField);
type G2AffineValue = (Fp2Value, Fp2Value);
type G2AffineConstant = (Fp2Constant, Fp2Constant);
type G2ProjectiveConstant = (Fp2Constant, Fp2Constant, Fp2Constant);

/// Returns the BN254 G2 twist coefficient `b = 3 / (u + 9)` in `Fq2(c0, c1)`.
///
/// # Panics
///
/// Panics if the hard-coded arkworks BN254 G2 twist coefficient fails to parse.
#[must_use]
pub fn g2_curve_coeff_b() -> (ForeignField, ForeignField) {
  (
    ForeignField::from_str_vartime(
      "19485874751759354771024239261021720505790618469301721065564631296452457478373",
    )
    .expect("hard-coded BN254 G2 coefficient b.c0 should parse"),
    ForeignField::from_str_vartime(
      "266929791119991161246907387137283842545076965332900288569378510910307636690",
    )
    .expect("hard-coded BN254 G2 coefficient b.c1 should parse"),
  )
}

fn g2_generator() -> G2AffineConstant {
  (
    (
      ForeignField::from_str_vartime(
        "10857046999023057135944570762232829481370756359578518086990519993285655852781",
      )
      .expect("hard-coded BN254 G2 generator x.c0 should parse"),
      ForeignField::from_str_vartime(
        "11559732032986387107991004021392285783925812861821192530917403151452391805634",
      )
      .expect("hard-coded BN254 G2 generator x.c1 should parse"),
    ),
    (
      ForeignField::from_str_vartime(
        "8495653923123431417604973247489272438418190587263600148770280649306958101930",
      )
      .expect("hard-coded BN254 G2 generator y.c0 should parse"),
      ForeignField::from_str_vartime(
        "4082367875863433681332203403145435568316851327593401208105741076214120093531",
      )
      .expect("hard-coded BN254 G2 generator y.c1 should parse"),
    ),
  )
}

fn fp2_add_constant(left: Fp2Constant, right: Fp2Constant) -> Fp2Constant {
  (left.0 + right.0, left.1 + right.1)
}

fn fp2_sub_constant(left: Fp2Constant, right: Fp2Constant) -> Fp2Constant {
  (left.0 - right.0, left.1 - right.1)
}

fn fp2_neg_constant(value: Fp2Constant) -> Fp2Constant {
  (-value.0, -value.1)
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
  let two_ab = ab + ab;

  (a_sq - b_sq, two_ab)
}

fn g2_projective_identity_constant() -> G2ProjectiveConstant {
  (
    (ForeignField::ONE, ForeignField::ZERO),
    (ForeignField::ONE, ForeignField::ZERO),
    (ForeignField::ZERO, ForeignField::ZERO),
  )
}

fn g2_projective_from_affine_constant(point: G2AffineConstant) -> G2ProjectiveConstant {
  (point.0, point.1, (ForeignField::ONE, ForeignField::ZERO))
}

fn g2_projective_double_constant(point: G2AffineConstant) -> G2ProjectiveConstant {
  let (x, y, z) = g2_projective_from_affine_constant(point);
  let a = fp2_square_constant(x);
  let b = fp2_square_constant(y);
  let c = fp2_square_constant(b);
  let d = {
    let x_plus_b = fp2_add_constant(x, b);
    let x_plus_b_sq = fp2_square_constant(x_plus_b);
    let d = fp2_sub_constant(fp2_sub_constant(x_plus_b_sq, a), c);
    fp2_add_constant(d, d)
  };
  let e = fp2_add_constant(fp2_add_constant(a, a), a);
  let f = fp2_square_constant(e);
  let x3 = fp2_sub_constant(f, fp2_add_constant(d, d));
  let y3 = {
    let e_times_d_minus_x3 = fp2_mul_constant(e, fp2_sub_constant(d, x3));
    let two_c = fp2_add_constant(c, c);
    let four_c = fp2_add_constant(two_c, two_c);
    let eight_c = fp2_add_constant(four_c, four_c);
    fp2_sub_constant(e_times_d_minus_x3, eight_c)
  };
  let yz = fp2_mul_constant(y, z);
  let z3 = fp2_add_constant(yz, yz);

  (x3, y3, z3)
}

fn g2_projective_add_constant(
  left: G2ProjectiveConstant,
  right: G2ProjectiveConstant,
) -> G2ProjectiveConstant {
  let (x1, y1, z1) = left;
  let (x2, y2, z2) = right;

  let z1z1 = fp2_square_constant(z1);
  let z2z2 = fp2_square_constant(z2);
  let u1 = fp2_mul_constant(x1, z2z2);
  let u2 = fp2_mul_constant(x2, z1z1);
  let s1 = fp2_mul_constant(y1, fp2_mul_constant(z2, z2z2));
  let s2 = fp2_mul_constant(y2, fp2_mul_constant(z1, z1z1));
  let h = fp2_sub_constant(u2, u1);
  let i = fp2_square_constant(fp2_add_constant(h, h));
  let j = fp2_mul_constant(h, i);
  let r = fp2_add_constant(fp2_sub_constant(s2, s1), fp2_sub_constant(s2, s1));
  let v = fp2_mul_constant(u1, i);
  let x3 = fp2_sub_constant(fp2_sub_constant(fp2_square_constant(r), j), fp2_add_constant(v, v));
  let y3 = {
    let r_times_v_minus_x3 = fp2_mul_constant(r, fp2_sub_constant(v, x3));
    let two_s1j = fp2_add_constant(fp2_mul_constant(s1, j), fp2_mul_constant(s1, j));
    fp2_sub_constant(r_times_v_minus_x3, two_s1j)
  };
  let z3 = {
    let z1_plus_z2 = fp2_add_constant(z1, z2);
    let z1_plus_z2_sq = fp2_square_constant(z1_plus_z2);
    let z3_pre = fp2_sub_constant(fp2_sub_constant(z1_plus_z2_sq, z1z1), z2z2);
    fp2_mul_constant(z3_pre, h)
  };

  (x3, y3, z3)
}

/// Assigned BN254 G2 affine point represented over the Fp2 twist coordinates.
///
/// This narrow slice supports only non-infinity affine points.
#[derive(Clone, Debug)]
pub struct AssignedG2Affine {
  /// X coordinate in Fp2.
  pub x: AssignedFp2,
  /// Y coordinate in Fp2.
  pub y: AssignedFp2,
}

impl AssignedG2Affine {
  /// Builds an assigned G2 affine point from assigned Fp2 coordinates.
  #[must_use]
  pub fn new(x: AssignedFp2, y: AssignedFp2) -> Self {
    Self { x, y }
  }

  /// Assigns a non-infinity G2 affine point from Fp2 coordinates.
  ///
  /// # Errors
  ///
  /// Returns an error if assigning any of the underlying Fp2 coordinates fails.
  pub fn assign(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    x: Fp2Value,
    y: Fp2Value,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      AssignedFp2::assign(chip, layouter, x.0, x.1)?,
      AssignedFp2::assign(chip, layouter, y.0, y.1)?,
    ))
  }

  /// Negates a G2 affine point inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if negating the underlying Fp2 y-coordinate fails.
  pub fn neg(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(self.x.clone(), self.y.neg(chip, layouter)?))
  }

  /// Asserts coordinate-wise equality against another assigned G2 affine point.
  ///
  /// # Errors
  ///
  /// Returns an error if either Fp2 coordinate equality constraint cannot be enforced.
  pub fn assert_equal(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<(), Error> {
    self.x.assert_equal(chip, layouter, &rhs.x)?;
    self.y.assert_equal(chip, layouter, &rhs.y)
  }

  /// Asserts that the assigned coordinates satisfy the BN254 G2 twist equation.
  ///
  /// # Errors
  ///
  /// Returns an error if any Fp2 assignment or equality constraint involved in the twist
  /// equation check fails.
  pub fn assert_on_curve(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let y_square = self.y.square(chip, layouter)?;
    let x_square = self.x.square(chip, layouter)?;
    let x_cube = x_square.mul(chip, layouter, &self.x)?;
    let coeff_b = {
      let coeff_b = g2_curve_coeff_b();
      AssignedFp2::assign(chip, layouter, Value::known(coeff_b.0), Value::known(coeff_b.1))?
    };
    let rhs = x_cube.add(chip, layouter, &coeff_b)?;

    y_square.assert_equal(chip, layouter, &rhs)
  }
}

/// Assigned BN254 G2 projective point in Jacobian coordinates `(X : Y : Z)`.
///
/// The represented affine point is `x = X / Z^2`, `y = Y / Z^3` for `Z != 0`.
/// This slice reserves `Z = 0` for the point at infinity and provides
/// [`AssignedG2Projective::identity`] for that encoding.
///
/// Arithmetic support is intentionally incomplete:
/// - `from_affine`, `neg`, and `double` are intended for non-identity inputs
/// - `add` implements the standard Jacobian-Jacobian formula `add-2007-bl`
/// - `add` does not support exceptional cases such as identity operands, `P = Q`,
///   or `P = -Q`, because this slice does not yet have circuit branching for those
///   cases
///
/// These constraints are deliberate for the current Week 2 projective slice.
#[derive(Clone, Debug)]
pub struct AssignedG2Projective {
  /// Jacobian X coordinate in Fp2.
  pub x: AssignedFp2,
  /// Jacobian Y coordinate in Fp2.
  pub y: AssignedFp2,
  /// Jacobian Z coordinate in Fp2.
  pub z: AssignedFp2,
}

impl AssignedG2Projective {
  /// Builds an assigned G2 projective point from assigned Fp2 coordinates.
  #[must_use]
  pub fn new(x: AssignedFp2, y: AssignedFp2, z: AssignedFp2) -> Self {
    Self { x, y, z }
  }

  /// Assigns the conventional Jacobian point-at-infinity representative `(1 : 1 : 0)`.
  ///
  /// # Errors
  ///
  /// Returns an error if assigning the fixed Fp2 constants fails.
  pub fn identity(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    let identity = g2_projective_identity_constant();
    Ok(Self::new(
      AssignedFp2::assign(chip, layouter, Value::known(identity.0.0), Value::known(identity.0.1))?,
      AssignedFp2::assign(chip, layouter, Value::known(identity.1.0), Value::known(identity.1.1))?,
      AssignedFp2::assign(chip, layouter, Value::known(identity.2.0), Value::known(identity.2.1))?,
    ))
  }

  /// Embeds a non-infinity affine point into Jacobian coordinates with `Z = 1`.
  ///
  /// # Errors
  ///
  /// Returns an error if assigning the Jacobian `Z = 1` coordinate fails.
  pub fn from_affine(
    affine: &AssignedG2Affine,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(affine.x.clone(), affine.y.clone(), AssignedFp2::one(chip, layouter)?))
  }

  /// Negates a non-identity projective point by flipping the Jacobian `Y` coordinate.
  ///
  /// # Errors
  ///
  /// Returns an error if negating the `Y` coordinate fails.
  pub fn neg(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(self.x.clone(), self.y.neg(chip, layouter)?, self.z.clone()))
  }

  /// Doubles a non-identity Jacobian point using the standard `dbl-2009-l` formula for `a = 0`.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 operation fails.
  pub fn double(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    let a = self.x.square(chip, layouter)?;
    let b = self.y.square(chip, layouter)?;
    let c = b.square(chip, layouter)?;
    let d = {
      let x_plus_b = self.x.add(chip, layouter, &b)?;
      let x_plus_b_sq = x_plus_b.square(chip, layouter)?;
      let d = x_plus_b_sq.sub(chip, layouter, &a)?.sub(chip, layouter, &c)?;
      d.add(chip, layouter, &d)?
    };
    let e = {
      let two_a = a.add(chip, layouter, &a)?;
      two_a.add(chip, layouter, &a)?
    };
    let f = e.square(chip, layouter)?;
    let x3 = {
      let two_d = d.add(chip, layouter, &d)?;
      f.sub(chip, layouter, &two_d)?
    };
    let y3 = {
      let d_minus_x3 = d.sub(chip, layouter, &x3)?;
      let e_times_d_minus_x3 = e.mul(chip, layouter, &d_minus_x3)?;
      let two_c = c.add(chip, layouter, &c)?;
      let four_c = two_c.add(chip, layouter, &two_c)?;
      let eight_c = four_c.add(chip, layouter, &four_c)?;
      e_times_d_minus_x3.sub(chip, layouter, &eight_c)?
    };
    let z3 = {
      let yz = self.y.mul(chip, layouter, &self.z)?;
      yz.add(chip, layouter, &yz)?
    };

    Ok(Self::new(x3, y3, z3))
  }

  /// Adds two non-identity Jacobian points using the standard incomplete `add-2007-bl` formula.
  ///
  /// Unsupported cases in this slice:
  /// - either operand is the identity (`Z = 0`)
  /// - the two points are equal, which should use doubling
  /// - the two points are negatives of each other, whose sum is the identity
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 operation fails.
  pub fn add(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    let z1z1 = self.z.square(chip, layouter)?;
    let z2z2 = rhs.z.square(chip, layouter)?;
    let u1 = self.x.mul(chip, layouter, &z2z2)?;
    let u2 = rhs.x.mul(chip, layouter, &z1z1)?;
    let s1 = {
      let z2_cubed = rhs.z.mul(chip, layouter, &z2z2)?;
      self.y.mul(chip, layouter, &z2_cubed)?
    };
    let s2 = {
      let z1_cubed = self.z.mul(chip, layouter, &z1z1)?;
      rhs.y.mul(chip, layouter, &z1_cubed)?
    };
    let h = u2.sub(chip, layouter, &u1)?;
    let i = {
      let two_h = h.add(chip, layouter, &h)?;
      two_h.square(chip, layouter)?
    };
    let j = h.mul(chip, layouter, &i)?;
    let r = {
      let s2_minus_s1 = s2.sub(chip, layouter, &s1)?;
      s2_minus_s1.add(chip, layouter, &s2_minus_s1)?
    };
    let v = u1.mul(chip, layouter, &i)?;
    let x3 = {
      let r_sq = r.square(chip, layouter)?;
      let two_v = v.add(chip, layouter, &v)?;
      r_sq.sub(chip, layouter, &j)?.sub(chip, layouter, &two_v)?
    };
    let y3 = {
      let v_minus_x3 = v.sub(chip, layouter, &x3)?;
      let r_times_v_minus_x3 = r.mul(chip, layouter, &v_minus_x3)?;
      let s1j = s1.mul(chip, layouter, &j)?;
      let two_s1j = s1j.add(chip, layouter, &s1j)?;
      r_times_v_minus_x3.sub(chip, layouter, &two_s1j)?
    };
    let z3 = {
      let z1_plus_z2 = self.z.add(chip, layouter, &rhs.z)?;
      let z1_plus_z2_sq = z1_plus_z2.square(chip, layouter)?;
      let z3_pre = z1_plus_z2_sq.sub(chip, layouter, &z1z1)?.sub(chip, layouter, &z2z2)?;
      z3_pre.mul(chip, layouter, &h)?
    };

    Ok(Self::new(x3, y3, z3))
  }

  /// Asserts coordinate-wise equality against another assigned G2 projective point.
  ///
  /// # Errors
  ///
  /// Returns an error if any Fp2 coordinate equality constraint cannot be enforced.
  pub fn assert_equal(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<(), Error> {
    self.x.assert_equal(chip, layouter, &rhs.x)?;
    self.y.assert_equal(chip, layouter, &rhs.y)?;
    self.z.assert_equal(chip, layouter, &rhs.z)
  }

  /// Asserts equality against a fixed projective constant.
  ///
  /// # Errors
  ///
  /// Returns an error if any Fp2 coordinate-equals-constant constraint fails.
  pub fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    expected: G2ProjectiveConstant,
  ) -> Result<(), Error> {
    self.x.assert_equal_to_fixed(chip, layouter, expected.0.0, expected.0.1)?;
    self.y.assert_equal_to_fixed(chip, layouter, expected.1.0, expected.1.1)?;
    self.z.assert_equal_to_fixed(chip, layouter, expected.2.0, expected.2.1)
  }

  /// Asserts that this Jacobian point represents the given non-infinity affine point.
  ///
  /// This avoids affine normalization by checking `X = x * Z^2` and `Y = y * Z^3`.
  /// It is intended for non-identity projective points only.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 operation or equality constraint fails.
  pub fn assert_equivalent_to_affine(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    expected: &AssignedG2Affine,
  ) -> Result<(), Error> {
    let z2 = self.z.square(chip, layouter)?;
    let z3 = self.z.mul(chip, layouter, &z2)?;
    let expected_x = expected.x.mul(chip, layouter, &z2)?;
    let expected_y = expected.y.mul(chip, layouter, &z3)?;

    self.x.assert_equal(chip, layouter, &expected_x)?;
    self.y.assert_equal(chip, layouter, &expected_y)
  }
}

/// Small circuit that asserts that a pair of Fp2 coordinates lies on BN254 G2.
#[derive(Clone, Debug)]
pub struct G2OnCurveCircuit {
  x: Fp2Value,
  y: Fp2Value,
}

impl G2OnCurveCircuit {
  /// Builds a new G2 on-curve circuit from affine Fp2 coordinates.
  #[must_use]
  pub fn new(x: Fp2Constant, y: Fp2Constant) -> Self {
    Self { x: (Value::known(x.0), Value::known(x.1)), y: (Value::known(y.0), Value::known(y.1)) }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let generator = g2_generator();
    Self::new(generator.0, generator.1)
  }
}

impl Default for G2OnCurveCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for G2OnCurveCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { x: (Value::unknown(), Value::unknown()), y: (Value::unknown(), Value::unknown()) }
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
    let point = AssignedG2Affine::assign(&chip, &mut layouter, self.x, self.y)?;
    point.assert_on_curve(&chip, &mut layouter)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises G2 affine negation and checks the result.
#[derive(Clone, Debug)]
pub struct G2NegCircuit {
  point: G2AffineValue,
  expected: G2AffineValue,
}

impl G2NegCircuit {
  /// Builds a new G2 negation circuit with a known expected output.
  #[must_use]
  pub fn new(point: G2AffineConstant, expected: G2AffineConstant) -> Self {
    Self {
      point: (
        (Value::known(point.0.0), Value::known(point.0.1)),
        (Value::known(point.1.0), Value::known(point.1.1)),
      ),
      expected: (
        (Value::known(expected.0.0), Value::known(expected.0.1)),
        (Value::known(expected.1.0), Value::known(expected.1.1)),
      ),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let point = g2_generator();
    Self::new(point, (point.0, fp2_neg_constant(point.1)))
  }
}

impl Default for G2NegCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for G2NegCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      point: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      expected: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
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
    let point = AssignedG2Affine::assign(&chip, &mut layouter, self.point.0, self.point.1)?;
    point.assert_on_curve(&chip, &mut layouter)?;
    let output = point.neg(&chip, &mut layouter)?;
    output.assert_on_curve(&chip, &mut layouter)?;
    let expected =
      AssignedG2Affine::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    output.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that embeds a non-infinity affine point into Jacobian coordinates.
#[derive(Clone, Debug)]
pub struct G2ProjectiveFromAffineCircuit {
  point: G2AffineValue,
}

impl G2ProjectiveFromAffineCircuit {
  /// Builds a new affine-to-projective embedding circuit.
  #[must_use]
  pub fn new(point: G2AffineConstant) -> Self {
    Self {
      point: (
        (Value::known(point.0.0), Value::known(point.0.1)),
        (Value::known(point.1.0), Value::known(point.1.1)),
      ),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new(g2_generator())
  }
}

impl Default for G2ProjectiveFromAffineCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for G2ProjectiveFromAffineCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { point: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())) }
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
    let affine = AssignedG2Affine::assign(&chip, &mut layouter, self.point.0, self.point.1)?;
    affine.assert_on_curve(&chip, &mut layouter)?;
    let projective = AssignedG2Projective::from_affine(&affine, &chip, &mut layouter)?;
    projective.assert_equivalent_to_affine(&chip, &mut layouter, &affine)?;
    projective.z.assert_equal_to_fixed(
      &chip,
      &mut layouter,
      ForeignField::ONE,
      ForeignField::ZERO,
    )?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that checks the conventional Jacobian identity encoding.
#[derive(Clone, Debug, Default)]
pub struct G2ProjectiveIdentityCircuit;

impl Circuit<NativeField> for G2ProjectiveIdentityCircuit {
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
    mut layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254FieldChip::new(&config);
    let identity = AssignedG2Projective::identity(&chip, &mut layouter)?;
    identity.assert_equal_to_fixed(&chip, &mut layouter, g2_projective_identity_constant())?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises projective G2 negation and checks affine equivalence.
#[derive(Clone, Debug)]
pub struct G2ProjectiveNegCircuit {
  point: G2AffineValue,
  expected: G2AffineValue,
}

impl G2ProjectiveNegCircuit {
  /// Builds a new projective G2 negation circuit with a known expected affine output.
  #[must_use]
  pub fn new(point: G2AffineConstant, expected: G2AffineConstant) -> Self {
    Self {
      point: (
        (Value::known(point.0.0), Value::known(point.0.1)),
        (Value::known(point.1.0), Value::known(point.1.1)),
      ),
      expected: (
        (Value::known(expected.0.0), Value::known(expected.0.1)),
        (Value::known(expected.1.0), Value::known(expected.1.1)),
      ),
    }
  }
}

impl Circuit<NativeField> for G2ProjectiveNegCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      point: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      expected: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
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
    let point = AssignedG2Affine::assign(&chip, &mut layouter, self.point.0, self.point.1)?;
    let expected =
      AssignedG2Affine::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    point.assert_on_curve(&chip, &mut layouter)?;
    expected.assert_on_curve(&chip, &mut layouter)?;
    let point_projective = AssignedG2Projective::from_affine(&point, &chip, &mut layouter)?;
    let output = point_projective.neg(&chip, &mut layouter)?;
    output.assert_equivalent_to_affine(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises projective G2 doubling and checks affine equivalence.
#[derive(Clone, Debug)]
pub struct G2ProjectiveDoubleCircuit {
  point: G2AffineValue,
  expected: G2AffineValue,
}

impl G2ProjectiveDoubleCircuit {
  /// Builds a new projective G2 doubling circuit with a known expected affine output.
  #[must_use]
  pub fn new(point: G2AffineConstant, expected: G2AffineConstant) -> Self {
    Self {
      point: (
        (Value::known(point.0.0), Value::known(point.0.1)),
        (Value::known(point.1.0), Value::known(point.1.1)),
      ),
      expected: (
        (Value::known(expected.0.0), Value::known(expected.0.1)),
        (Value::known(expected.1.0), Value::known(expected.1.1)),
      ),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let point = g2_generator();
    let doubled = g2_projective_double_constant(point);

    Self::new(point, (doubled.0, doubled.1))
  }
}

impl Default for G2ProjectiveDoubleCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for G2ProjectiveDoubleCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      point: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      expected: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
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
    let point = AssignedG2Affine::assign(&chip, &mut layouter, self.point.0, self.point.1)?;
    let expected =
      AssignedG2Affine::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    point.assert_on_curve(&chip, &mut layouter)?;
    expected.assert_on_curve(&chip, &mut layouter)?;
    let point_projective = AssignedG2Projective::from_affine(&point, &chip, &mut layouter)?;
    let output = point_projective.double(&chip, &mut layouter)?;
    output.assert_equivalent_to_affine(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises projective G2 addition and checks affine equivalence.
#[derive(Clone, Debug)]
pub struct G2ProjectiveAddCircuit {
  left: G2AffineValue,
  right: G2AffineValue,
  expected: G2AffineValue,
}

impl G2ProjectiveAddCircuit {
  /// Builds a new projective G2 addition circuit with a known expected affine output.
  #[must_use]
  pub fn new(left: G2AffineConstant, right: G2AffineConstant, expected: G2AffineConstant) -> Self {
    Self {
      left: (
        (Value::known(left.0.0), Value::known(left.0.1)),
        (Value::known(left.1.0), Value::known(left.1.1)),
      ),
      right: (
        (Value::known(right.0.0), Value::known(right.0.1)),
        (Value::known(right.1.0), Value::known(right.1.1)),
      ),
      expected: (
        (Value::known(expected.0.0), Value::known(expected.0.1)),
        (Value::known(expected.1.0), Value::known(expected.1.1)),
      ),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let left = g2_generator();
    let doubled = g2_projective_double_constant(left);
    let right = (doubled.0, doubled.1);
    let added = g2_projective_add_constant(
      g2_projective_from_affine_constant(left),
      g2_projective_from_affine_constant(right),
    );

    Self::new(left, right, (added.0, added.1))
  }
}

impl Default for G2ProjectiveAddCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for G2ProjectiveAddCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      left: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      right: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      expected: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
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
    let left = AssignedG2Affine::assign(&chip, &mut layouter, self.left.0, self.left.1)?;
    let right = AssignedG2Affine::assign(&chip, &mut layouter, self.right.0, self.right.1)?;
    let expected =
      AssignedG2Affine::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    left.assert_on_curve(&chip, &mut layouter)?;
    right.assert_on_curve(&chip, &mut layouter)?;
    expected.assert_on_curve(&chip, &mut layouter)?;
    let left_projective = AssignedG2Projective::from_affine(&left, &chip, &mut layouter)?;
    let right_projective = AssignedG2Projective::from_affine(&right, &chip, &mut layouter)?;
    let output = left_projective.add(&chip, &mut layouter, &right_projective)?;
    output.assert_equivalent_to_affine(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}
