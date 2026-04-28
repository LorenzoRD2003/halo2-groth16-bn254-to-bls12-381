use ff::{Field, PrimeField};
use midnight_circuits::field::foreign::params::{FieldEmulationParams, MultiEmulationParams};
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{
  AssignedFp2, AssignedG2Affine, Bn254FieldChip, Bn254FieldConfig, ForeignField, G2AffineConstant,
  G2AffineValue, G2ProjectiveConstant, NativeField, g2_affine_from_projective_constant,
  g2_generator, g2_projective_add_constant, g2_projective_double_constant,
  g2_projective_from_affine_constant, g2_projective_identity_constant,
};

fn double_step_jacobian<FHost>(
  point: &AssignedG2Projective<FHost>,
  chip: &Bn254FieldChip<FHost>,
  layouter: &mut impl Layouter<FHost>,
) -> Result<(AssignedFp2<FHost>, AssignedFp2<FHost>, AssignedFp2<FHost>), Error>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  let x_square = point.x.square(chip, layouter)?;
  let y_square = point.y.square(chip, layouter)?;
  let y_fourth = y_square.square(chip, layouter)?;
  let slope_intermediate = {
    let x_plus_y_square = point.x.add(chip, layouter, &y_square)?;
    let x_plus_y_square_sq = x_plus_y_square.square(chip, layouter)?;
    let slope_intermediate =
      x_plus_y_square_sq.sub(chip, layouter, &x_square)?.sub(chip, layouter, &y_fourth)?;
    slope_intermediate.add(chip, layouter, &slope_intermediate)?
  };
  let slope = {
    let two_x_square = x_square.add(chip, layouter, &x_square)?;
    two_x_square.add(chip, layouter, &x_square)?
  };
  let slope_square = slope.square(chip, layouter)?;
  let next_x = {
    let two_slope_intermediate = slope_intermediate.add(chip, layouter, &slope_intermediate)?;
    slope_square.sub(chip, layouter, &two_slope_intermediate)?
  };
  let next_y = {
    let delta = slope_intermediate.sub(chip, layouter, &next_x)?;
    let slope_times_delta = slope.mul(chip, layouter, &delta)?;
    let two_y_fourth = y_fourth.add(chip, layouter, &y_fourth)?;
    let four_y_fourth = two_y_fourth.add(chip, layouter, &two_y_fourth)?;
    let eight_y_fourth = four_y_fourth.add(chip, layouter, &four_y_fourth)?;
    slope_times_delta.sub(chip, layouter, &eight_y_fourth)?
  };
  let next_z = {
    let yz = point.y.mul(chip, layouter, &point.z)?;
    yz.add(chip, layouter, &yz)?
  };

  Ok((next_x, next_y, next_z))
}

fn add_step_jacobian<FHost>(
  left: &AssignedG2Projective<FHost>,
  right: &AssignedG2Projective<FHost>,
  chip: &Bn254FieldChip<FHost>,
  layouter: &mut impl Layouter<FHost>,
) -> Result<(AssignedFp2<FHost>, AssignedFp2<FHost>, AssignedFp2<FHost>), Error>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  let z1z1 = left.z.square(chip, layouter)?;
  let z2z2 = right.z.square(chip, layouter)?;
  let u1 = left.x.mul(chip, layouter, &z2z2)?;
  let u2 = right.x.mul(chip, layouter, &z1z1)?;
  let s1 = {
    let z2_cubed = right.z.mul(chip, layouter, &z2z2)?;
    left.y.mul(chip, layouter, &z2_cubed)?
  };
  let s2 = {
    let z1_cubed = left.z.mul(chip, layouter, &z1z1)?;
    right.y.mul(chip, layouter, &z1_cubed)?
  };
  let x_diff = u2.sub(chip, layouter, &u1)?;
  let x_diff_twice_sq = {
    let two_x_diff = x_diff.add(chip, layouter, &x_diff)?;
    two_x_diff.square(chip, layouter)?
  };
  let x_diff_cubed_scaled = x_diff.mul(chip, layouter, &x_diff_twice_sq)?;
  let y_diff_twice = {
    let s2_minus_s1 = s2.sub(chip, layouter, &s1)?;
    s2_minus_s1.add(chip, layouter, &s2_minus_s1)?
  };
  let u1_times_scale = u1.mul(chip, layouter, &x_diff_twice_sq)?;
  let next_x = {
    let y_diff_twice_sq = y_diff_twice.square(chip, layouter)?;
    let two_u1_times_scale = u1_times_scale.add(chip, layouter, &u1_times_scale)?;
    y_diff_twice_sq.sub(chip, layouter, &x_diff_cubed_scaled)?.sub(
      chip,
      layouter,
      &two_u1_times_scale,
    )?
  };
  let next_y = {
    let delta = u1_times_scale.sub(chip, layouter, &next_x)?;
    let y_slope_times_delta = y_diff_twice.mul(chip, layouter, &delta)?;
    let s1_scaled = s1.mul(chip, layouter, &x_diff_cubed_scaled)?;
    let two_s1_scaled = s1_scaled.add(chip, layouter, &s1_scaled)?;
    y_slope_times_delta.sub(chip, layouter, &two_s1_scaled)?
  };
  let next_z = {
    let z1_plus_z2 = left.z.add(chip, layouter, &right.z)?;
    let z1_plus_z2_sq = z1_plus_z2.square(chip, layouter)?;
    let z3_pre = z1_plus_z2_sq.sub(chip, layouter, &z1z1)?.sub(chip, layouter, &z2z2)?;
    z3_pre.mul(chip, layouter, &x_diff)?
  };

  Ok((next_x, next_y, next_z))
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
pub struct AssignedG2Projective<FHost = NativeField>
where
  FHost: PrimeField,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  /// Jacobian X coordinate in Fp2.
  pub x: AssignedFp2<FHost>,
  /// Jacobian Y coordinate in Fp2.
  pub y: AssignedFp2<FHost>,
  /// Jacobian Z coordinate in Fp2.
  pub z: AssignedFp2<FHost>,
}

impl<FHost> AssignedG2Projective<FHost>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  /// Builds an assigned G2 projective point from assigned Fp2 coordinates.
  #[must_use]
  pub fn new(x: AssignedFp2<FHost>, y: AssignedFp2<FHost>, z: AssignedFp2<FHost>) -> Self {
    Self { x, y, z }
  }

  /// Assigns the conventional Jacobian point-at-infinity representative `(1 : 1 : 0)`.
  ///
  /// # Errors
  ///
  /// Returns an error if assigning the fixed Fp2 constants fails.
  pub fn identity(
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    let identity = g2_projective_identity_constant();
    Ok(Self::new(
      AssignedFp2::<FHost>::assign(
        chip,
        layouter,
        Value::known(identity.0.0),
        Value::known(identity.0.1),
      )?,
      AssignedFp2::<FHost>::assign(
        chip,
        layouter,
        Value::known(identity.1.0),
        Value::known(identity.1.1),
      )?,
      AssignedFp2::<FHost>::assign(
        chip,
        layouter,
        Value::known(identity.2.0),
        Value::known(identity.2.1),
      )?,
    ))
  }

  /// Embeds a non-infinity affine point into Jacobian coordinates with `Z = 1`.
  ///
  /// # Errors
  ///
  /// Returns an error if assigning the Jacobian `Z = 1` coordinate fails.
  pub fn from_affine(
    affine: &AssignedG2Affine<FHost>,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    Ok(Self::new(affine.x.clone(), affine.y.clone(), AssignedFp2::<FHost>::one(chip, layouter)?))
  }

  /// Negates a non-identity projective point by flipping the Jacobian `Y` coordinate.
  ///
  /// # Errors
  ///
  /// Returns an error if negating the `Y` coordinate fails.
  pub fn neg(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
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
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
  ) -> Result<Self, Error> {
    let (next_x, next_y, next_z) = double_step_jacobian(self, chip, layouter)?;
    Ok(Self::new(next_x, next_y, next_z))
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
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    let (next_x, next_y, next_z) = add_step_jacobian(self, rhs, chip, layouter)?;
    Ok(Self::new(next_x, next_y, next_z))
  }

  /// Asserts coordinate-wise equality against another assigned G2 projective point.
  ///
  /// # Errors
  ///
  /// Returns an error if any Fp2 coordinate equality constraint cannot be enforced.
  pub fn assert_equal(
    &self,
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
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
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
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
    chip: &Bn254FieldChip<FHost>,
    layouter: &mut impl Layouter<FHost>,
    expected: &AssignedG2Affine<FHost>,
  ) -> Result<(), Error> {
    let z2 = self.z.square(chip, layouter)?;
    let z3 = self.z.mul(chip, layouter, &z2)?;
    let expected_x = expected.x.mul(chip, layouter, &z2)?;
    let expected_y = expected.y.mul(chip, layouter, &z3)?;

    self.x.assert_equal(chip, layouter, &expected_x)?;
    self.y.assert_equal(chip, layouter, &expected_y)
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

    Self::new(point, g2_affine_from_projective_constant(doubled))
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
    let right = g2_affine_from_projective_constant(doubled);
    let added = g2_projective_add_constant(
      g2_projective_from_affine_constant(left),
      g2_projective_from_affine_constant(right),
    );

    Self::new(left, right, g2_affine_from_projective_constant(added))
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
