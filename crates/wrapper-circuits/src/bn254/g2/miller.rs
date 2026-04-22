use ff::Field;
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{
  AssignedFp, AssignedFp2, AssignedFp6, AssignedFp12, AssignedG2Affine, Bn254FieldChip,
  Bn254FieldConfig, ForeignField, Fp2Value, G2AffineConstant, G2AffineValue, G2LineCoeffsConstant,
  G2MillerPointConstant, NativeField, g2_affine_from_miller_point_constant, g2_curve_coeff_b,
  g2_generator, g2_miller_double_with_line_constant, g2_miller_mixed_add_with_line_constant,
  g2_miller_point_from_affine_constant,
};
#[cfg(test)]
use super::{Fp12Constant, Fp12Value, G2LineCoeffsValue};

type MillerAccumulatorFixed = (
  ((ForeignField, ForeignField), (ForeignField, ForeignField), (ForeignField, ForeignField)),
  ((ForeignField, ForeignField), (ForeignField, ForeignField), (ForeignField, ForeignField)),
);

/// Sparse BN254 line coefficients already shaped for the later D-twist Miller loop.
///
/// The future accumulator will evaluate these coefficients at a G1 affine point
/// `(x_P, y_P)` into the sparse Fp12 element
/// `ell_0 * y_P + ell_w * x_P * w + ell_vw * v * w`.
///
/// This matches the BN254 D-twist sparse layout consumed by `mul_by_034`
/// in arkworks / halo2curves / Midnight:
/// - `ell_0 * y_P` lands in Fp12 slot `c0`
/// - `ell_w * x_P` lands in Fp12 slot `c3`
/// - `ell_vw` lands in Fp12 slot `c4`
#[derive(Clone, Debug)]
pub struct AssignedG2LineCoeffs {
  /// Coefficient scaled later by the G1 affine `y` coordinate and embedded into Fp12 slot `c0`.
  pub ell_0: AssignedFp2,
  /// Coefficient scaled later by the G1 affine `x` coordinate and embedded into Fp12 slot `c3`.
  pub ell_w: AssignedFp2,
  /// Constant coefficient embedded directly into Fp12 slot `c4 = v * w`.
  pub ell_vw: AssignedFp2,
}

impl AssignedG2LineCoeffs {
  /// Builds line coefficients from their three assigned Fp2 coordinates.
  #[must_use]
  pub fn new(
    constant_term_coeff: AssignedFp2,
    x_slot_coeff: AssignedFp2,
    vw_slot_coeff: AssignedFp2,
  ) -> Self {
    Self { ell_0: constant_term_coeff, ell_w: x_slot_coeff, ell_vw: vw_slot_coeff }
  }

  /// Assigns line coefficients from three Fp2 witnesses.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 assignment fails.
  pub fn assign(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    constant_term_value: Fp2Value,
    x_slot_value: Fp2Value,
    vw_slot_value: Fp2Value,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      AssignedFp2::assign(chip, layouter, constant_term_value.0, constant_term_value.1)?,
      AssignedFp2::assign(chip, layouter, x_slot_value.0, x_slot_value.1)?,
      AssignedFp2::assign(chip, layouter, vw_slot_value.0, vw_slot_value.1)?,
    ))
  }

  /// Asserts coordinate-wise equality against another assigned line-coefficient tuple.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 equality constraint fails.
  pub fn assert_equal(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<(), Error> {
    self.ell_0.assert_equal(chip, layouter, &rhs.ell_0)?;
    self.ell_w.assert_equal(chip, layouter, &rhs.ell_w)?;
    self.ell_vw.assert_equal(chip, layouter, &rhs.ell_vw)
  }

  /// Asserts equality against fixed line coefficients.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 coordinate-equals-constant constraint fails.
  pub fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    expected: G2LineCoeffsConstant,
  ) -> Result<(), Error> {
    self.ell_0.assert_equal_to_fixed(chip, layouter, expected.0.0, expected.0.1)?;
    self.ell_w.assert_equal_to_fixed(chip, layouter, expected.1.0, expected.1.1)?;
    self.ell_vw.assert_equal_to_fixed(chip, layouter, expected.2.0, expected.2.1)
  }
}

fn evaluate_sparse_line_at_g1(
  line: &AssignedG2LineCoeffs,
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  g1_x: &AssignedFp,
  g1_y: &AssignedFp,
) -> Result<AssignedFp12, Error> {
  let slot_c0 = line.ell_0.scale_by_fp(chip, layouter, g1_y)?;
  let slot_c3 = line.ell_w.scale_by_fp(chip, layouter, g1_x)?;
  let zero_fp2 = AssignedFp2::zero(chip, layouter)?;

  Ok(AssignedFp12::new(
    AssignedFp6::new(slot_c0, zero_fp2.clone(), zero_fp2.clone()),
    AssignedFp6::new(slot_c3, line.ell_vw.clone(), zero_fp2),
  ))
}

fn double_step_hom_projective(
  point: &AssignedG2MillerPoint,
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
) -> Result<(AssignedG2MillerPoint, AssignedG2LineCoeffs), Error> {
  let two_inv = chip.assign(
    layouter,
    Value::known(
      ForeignField::from(2_u64)
        .invert()
        .expect("hard-coded BN254 base-field two should be invertible"),
    ),
  )?;

  let xy = point.x.mul(chip, layouter, &point.y)?;
  let xy_half = xy.scale_by_fp(chip, layouter, &two_inv)?;
  let y_square = point.y.square(chip, layouter)?;
  let z_square = point.z.square(chip, layouter)?;
  let three_z_square = z_square.add(chip, layouter, &z_square)?.add(chip, layouter, &z_square)?;
  let coeff_b = g2_curve_coeff_b();
  let twist_b =
    AssignedFp2::assign(chip, layouter, Value::known(coeff_b.0), Value::known(coeff_b.1))?;
  let twist_times_three_z_square = twist_b.mul(chip, layouter, &three_z_square)?;
  let triple_twist_term = twist_times_three_z_square
    .add(chip, layouter, &twist_times_three_z_square)?
    .add(chip, layouter, &twist_times_three_z_square)?;
  let average_y_square_and_twist =
    y_square.add(chip, layouter, &triple_twist_term)?.scale_by_fp(chip, layouter, &two_inv)?;
  let y_plus_z = point.y.add(chip, layouter, &point.z)?;
  let y_plus_z_sq = y_plus_z.square(chip, layouter)?;
  let y_plus_z_sum = y_square.add(chip, layouter, &z_square)?;
  let y_minus_twist_term = y_square.sub(chip, layouter, &triple_twist_term)?;
  let vertical_term = twist_times_three_z_square.sub(chip, layouter, &y_square)?;
  let x_square = point.x.square(chip, layouter)?;
  let twist_term_square = twist_times_three_z_square.square(chip, layouter)?;

  let y_plus_z_cross = y_plus_z_sq.sub(chip, layouter, &y_plus_z_sum)?;
  let next_x = xy_half.mul(chip, layouter, &y_minus_twist_term)?;
  let three_twist_term_square = twist_term_square.add(chip, layouter, &twist_term_square)?.add(
    chip,
    layouter,
    &twist_term_square,
  )?;
  let next_y = average_y_square_and_twist.square(chip, layouter)?.sub(
    chip,
    layouter,
    &three_twist_term_square,
  )?;
  let next_z = y_square.mul(chip, layouter, &y_plus_z_cross)?;

  let line = AssignedG2LineCoeffs::new(
    y_plus_z_cross.neg(chip, layouter)?,
    x_square.add(chip, layouter, &x_square)?.add(chip, layouter, &x_square)?,
    vertical_term,
  );

  Ok((AssignedG2MillerPoint::new(next_x, next_y, next_z), line))
}

fn mixed_add_step_hom_projective(
  point: &AssignedG2MillerPoint,
  addend: &AssignedG2Affine,
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
) -> Result<(AssignedG2MillerPoint, AssignedG2LineCoeffs), Error> {
  let rhs_y_times_z = addend.y.mul(chip, layouter, &point.z)?;
  let rhs_x_times_z = addend.x.mul(chip, layouter, &point.z)?;
  let theta = point.y.sub(chip, layouter, &rhs_y_times_z)?;
  let lambda = point.x.sub(chip, layouter, &rhs_x_times_z)?;
  let theta_square = theta.square(chip, layouter)?;
  let lambda_square = lambda.square(chip, layouter)?;
  let lambda_cubed = lambda.mul(chip, layouter, &lambda_square)?;
  let z_times_theta_square = point.z.mul(chip, layouter, &theta_square)?;
  let x_times_lambda_square = point.x.mul(chip, layouter, &lambda_square)?;
  let double_x_lambda_square = x_times_lambda_square.add(chip, layouter, &x_times_lambda_square)?;
  let next_x_intermediate = lambda_cubed.add(chip, layouter, &z_times_theta_square)?.sub(
    chip,
    layouter,
    &double_x_lambda_square,
  )?;

  let next_x = lambda.mul(chip, layouter, &next_x_intermediate)?;
  let x_delta = x_times_lambda_square.sub(chip, layouter, &next_x_intermediate)?;
  let theta_times_delta = theta.mul(chip, layouter, &x_delta)?;
  let lambda_cubed_times_y = lambda_cubed.mul(chip, layouter, &point.y)?;
  let next_y = theta_times_delta.sub(chip, layouter, &lambda_cubed_times_y)?;
  let next_z = point.z.mul(chip, layouter, &lambda_cubed)?;
  let theta_times_rhs_x = theta.mul(chip, layouter, &addend.x)?;
  let lambda_times_rhs_y = lambda.mul(chip, layouter, &addend.y)?;
  let line_constant_term = theta_times_rhs_x.sub(chip, layouter, &lambda_times_rhs_y)?;

  let line =
    AssignedG2LineCoeffs::new(lambda.clone(), theta.neg(chip, layouter)?, line_constant_term);

  Ok((AssignedG2MillerPoint::new(next_x, next_y, next_z), line))
}

/// Dedicated Miller-loop accumulator over BN254 Fp12.
///
/// This type is the public consumption boundary for `AssignedG2LineCoeffs`.
/// It keeps Miller-step semantics out of `AssignedFp12`: callers multiply an
/// accumulator by a line evaluation rather than asking the line coefficients to
/// materialize a generic Fp12 value directly.
#[derive(Clone, Debug)]
pub struct AssignedMillerAccumulator {
  value: AssignedFp12,
}

impl AssignedMillerAccumulator {
  /// Builds an accumulator from an assigned Fp12 value.
  #[must_use]
  pub fn new(value: AssignedFp12) -> Self {
    Self { value }
  }

  /// Initializes the Miller accumulator to multiplicative identity.
  ///
  /// # Errors
  ///
  /// Returns an error if assigning the underlying Fp12 identity fails.
  pub fn one(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(AssignedFp12::one(chip, layouter)?))
  }

  /// Multiplies the accumulator by the sparse evaluation of a G2 line at a G1 affine point.
  ///
  /// This is the intended public boundary between line extraction and later Miller-loop work.
  ///
  /// # Errors
  ///
  /// Returns an error if the sparse line evaluation or Fp12 multiplication fails.
  pub fn mul_by_line(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    line: &AssignedG2LineCoeffs,
    g1_x: &AssignedFp,
    g1_y: &AssignedFp,
  ) -> Result<Self, Error> {
    let line_value = evaluate_sparse_line_at_g1(line, chip, layouter, g1_x, g1_y)?;
    Ok(Self::new(self.value.mul(chip, layouter, &line_value)?))
  }

  /// Asserts equality against a fixed Fp12 constant.
  ///
  /// # Errors
  ///
  /// Returns an error if the underlying Fp12 equality-to-fixed check fails.
  pub fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    expected: MillerAccumulatorFixed,
  ) -> Result<(), Error> {
    self.value.assert_equal_to_fixed(chip, layouter, expected)
  }
}

/// Miller-step G2 state in homogeneous projective coordinates `(X : Y : Z)`.
///
/// This is intentionally separate from [`AssignedG2Projective`], which models
/// Jacobian arithmetic for the narrow general-purpose G2 slice. The BN254 line
/// extraction path follows the homogeneous-projective state used by the
/// arkworks / Midnight prepared-G2 pipeline, because that yields Miller-ready
/// line coefficients without introducing another conversion layer later.
///
/// The represented affine point is `x = X / Z`, `y = Y / Z` for `Z != 0`.
/// Identity handling is intentionally out of scope for this slice.
#[derive(Clone, Debug)]
pub struct AssignedG2MillerPoint {
  /// Homogeneous X coordinate in Fp2.
  pub x: AssignedFp2,
  /// Homogeneous Y coordinate in Fp2.
  pub y: AssignedFp2,
  /// Homogeneous Z coordinate in Fp2.
  pub z: AssignedFp2,
}

impl AssignedG2MillerPoint {
  /// Builds a Miller-step point from assigned Fp2 coordinates.
  #[must_use]
  pub fn new(x: AssignedFp2, y: AssignedFp2, z: AssignedFp2) -> Self {
    Self { x, y, z }
  }

  /// Assigns a Miller-step point from three Fp2 coordinates.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 assignment fails.
  pub fn assign(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    x: Fp2Value,
    y: Fp2Value,
    z: Fp2Value,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      AssignedFp2::assign(chip, layouter, x.0, x.1)?,
      AssignedFp2::assign(chip, layouter, y.0, y.1)?,
      AssignedFp2::assign(chip, layouter, z.0, z.1)?,
    ))
  }

  /// Initializes the Miller-step state from a non-infinity G2 affine point with `Z = 1`.
  ///
  /// # Errors
  ///
  /// Returns an error if assigning the homogeneous `Z = 1` coordinate fails.
  pub fn from_affine(
    affine: &AssignedG2Affine,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(affine.x.clone(), affine.y.clone(), AssignedFp2::one(chip, layouter)?))
  }

  /// Performs a Miller-path doubling step and returns both the next point and its line coefficients.
  ///
  /// This implements the BN homogeneous-projective doubling formulas used by
  /// arkworks prepared-G2 generation and described in
  /// <https://eprint.iacr.org/2013/722.pdf>.
  ///
  /// Unsupported in this slice:
  /// - identity inputs (`Z = 0`)
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 operation fails.
  pub fn double_with_line(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<(Self, AssignedG2LineCoeffs), Error> {
    double_step_hom_projective(self, chip, layouter)
  }

  /// Performs a Miller-path mixed-addition step against a non-infinity affine addend.
  ///
  /// This implements the BN homogeneous-projective mixed-add formulas used by
  /// arkworks prepared-G2 generation.
  ///
  /// Unsupported in this slice:
  /// - identity current point (`Z = 0`)
  /// - `self` equal to `rhs`
  /// - `self` equal to `-rhs`
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 operation fails.
  pub fn mixed_add_with_line(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &AssignedG2Affine,
  ) -> Result<(Self, AssignedG2LineCoeffs), Error> {
    mixed_add_step_hom_projective(self, rhs, chip, layouter)
  }

  /// Asserts coordinate-wise equality against fixed homogeneous-projective coordinates.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp2 coordinate-equals-constant constraint fails.
  pub fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    expected: G2MillerPointConstant,
  ) -> Result<(), Error> {
    self.x.assert_equal_to_fixed(chip, layouter, expected.0.0, expected.0.1)?;
    self.y.assert_equal_to_fixed(chip, layouter, expected.1.0, expected.1.1)?;
    self.z.assert_equal_to_fixed(chip, layouter, expected.2.0, expected.2.1)
  }

  /// Asserts that this homogeneous point represents the given non-infinity affine point.
  ///
  /// This checks `X = x * Z` and `Y = y * Z`.
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
    let expected_x = expected.x.mul(chip, layouter, &self.z)?;
    let expected_y = expected.y.mul(chip, layouter, &self.z)?;
    self.x.assert_equal(chip, layouter, &expected_x)?;
    self.y.assert_equal(chip, layouter, &expected_y)
  }
}

/// Small circuit that exercises a BN254 Miller-path G2 doubling step and checks the line output.
#[derive(Clone, Debug)]
pub struct G2DoubleWithLineCircuit {
  point: G2AffineValue,
  expected_point: G2AffineValue,
  expected_line: G2LineCoeffsConstant,
}

impl G2DoubleWithLineCircuit {
  /// Builds a new Miller-path doubling circuit with known affine and line outputs.
  #[must_use]
  pub fn new(
    point: G2AffineConstant,
    expected_point: G2AffineConstant,
    expected_line: G2LineCoeffsConstant,
  ) -> Self {
    Self {
      point: (
        (Value::known(point.0.0), Value::known(point.0.1)),
        (Value::known(point.1.0), Value::known(point.1.1)),
      ),
      expected_point: (
        (Value::known(expected_point.0.0), Value::known(expected_point.0.1)),
        (Value::known(expected_point.1.0), Value::known(expected_point.1.1)),
      ),
      expected_line,
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let point = g2_generator();
    let (next_point, line) =
      g2_miller_double_with_line_constant(g2_miller_point_from_affine_constant(point));
    Self::new(point, g2_affine_from_miller_point_constant(next_point), line)
  }
}

impl Default for G2DoubleWithLineCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for G2DoubleWithLineCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      point: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      expected_point: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      expected_line: self.expected_line,
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
    let expected_point =
      AssignedG2Affine::assign(&chip, &mut layouter, self.expected_point.0, self.expected_point.1)?;
    point.assert_on_curve(&chip, &mut layouter)?;
    expected_point.assert_on_curve(&chip, &mut layouter)?;
    let miller_point = AssignedG2MillerPoint::from_affine(&point, &chip, &mut layouter)?;
    let (next_point, line) = miller_point.double_with_line(&chip, &mut layouter)?;
    next_point.assert_equivalent_to_affine(&chip, &mut layouter, &expected_point)?;
    line.assert_equal_to_fixed(&chip, &mut layouter, self.expected_line)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises a BN254 Miller-path mixed-add step and checks the line output.
#[derive(Clone, Debug)]
pub struct G2MixedAddWithLineCircuit {
  point: G2ProjectiveValue,
  addend: G2AffineValue,
  expected_point: G2AffineValue,
  expected_line: G2LineCoeffsConstant,
}

type G2ProjectiveValue = (Fp2Value, Fp2Value, Fp2Value);

impl G2MixedAddWithLineCircuit {
  /// Builds a new Miller-path mixed-add circuit with known affine and line outputs.
  #[must_use]
  pub fn new(
    point: G2MillerPointConstant,
    addend: G2AffineConstant,
    expected_point: G2AffineConstant,
    expected_line: G2LineCoeffsConstant,
  ) -> Self {
    Self {
      point: (
        (Value::known(point.0.0), Value::known(point.0.1)),
        (Value::known(point.1.0), Value::known(point.1.1)),
        (Value::known(point.2.0), Value::known(point.2.1)),
      ),
      addend: (
        (Value::known(addend.0.0), Value::known(addend.0.1)),
        (Value::known(addend.1.0), Value::known(addend.1.1)),
      ),
      expected_point: (
        (Value::known(expected_point.0.0), Value::known(expected_point.0.1)),
        (Value::known(expected_point.1.0), Value::known(expected_point.1.1)),
      ),
      expected_line,
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let generator = g2_generator();
    let doubled_state =
      g2_miller_double_with_line_constant(g2_miller_point_from_affine_constant(generator)).0;
    let (next_point, line) = g2_miller_mixed_add_with_line_constant(doubled_state, generator);
    Self::new(doubled_state, generator, g2_affine_from_miller_point_constant(next_point), line)
  }
}

impl Default for G2MixedAddWithLineCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for G2MixedAddWithLineCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      point: (
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
      ),
      addend: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      expected_point: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      expected_line: self.expected_line,
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
    let point = AssignedG2MillerPoint::assign(
      &chip,
      &mut layouter,
      self.point.0,
      self.point.1,
      self.point.2,
    )?;
    let addend = AssignedG2Affine::assign(&chip, &mut layouter, self.addend.0, self.addend.1)?;
    let expected_point =
      AssignedG2Affine::assign(&chip, &mut layouter, self.expected_point.0, self.expected_point.1)?;
    addend.assert_on_curve(&chip, &mut layouter)?;
    expected_point.assert_on_curve(&chip, &mut layouter)?;
    let (next_point, line) = point.mixed_add_with_line(&chip, &mut layouter, &addend)?;
    next_point.assert_equivalent_to_affine(&chip, &mut layouter, &expected_point)?;
    line.assert_equal_to_fixed(&chip, &mut layouter, self.expected_line)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises Miller-accumulator multiplication by one sparse G2 line.
#[cfg(test)]
#[derive(Clone, Debug)]
pub struct G2MulByLineCircuit {
  line: G2LineCoeffsValue,
  g1_x: Value<ForeignField>,
  g1_y: Value<ForeignField>,
  expected: Fp12Value,
}

#[cfg(test)]
impl G2MulByLineCircuit {
  /// Builds a new Miller-accumulator line-multiplication circuit with a known Fp12 output.
  #[must_use]
  pub fn new(
    line: G2LineCoeffsConstant,
    g1_x: ForeignField,
    g1_y: ForeignField,
    expected: &Fp12Constant,
  ) -> Self {
    Self {
      line: (
        (Value::known(line.0.0), Value::known(line.0.1)),
        (Value::known(line.1.0), Value::known(line.1.1)),
        (Value::known(line.2.0), Value::known(line.2.1)),
      ),
      g1_x: Value::known(g1_x),
      g1_y: Value::known(g1_y),
      expected: (
        (
          (Value::known(expected.0.0.0), Value::known(expected.0.0.1)),
          (Value::known(expected.0.1.0), Value::known(expected.0.1.1)),
          (Value::known(expected.0.2.0), Value::known(expected.0.2.1)),
        ),
        (
          (Value::known(expected.1.0.0), Value::known(expected.1.0.1)),
          (Value::known(expected.1.1.0), Value::known(expected.1.1.1)),
          (Value::known(expected.1.2.0), Value::known(expected.1.2.1)),
        ),
      ),
    }
  }
}

#[cfg(test)]
impl Circuit<NativeField> for G2MulByLineCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      line: (
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
      ),
      g1_x: Value::unknown(),
      g1_y: Value::unknown(),
      expected: (
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
    let line =
      AssignedG2LineCoeffs::assign(&chip, &mut layouter, self.line.0, self.line.1, self.line.2)?;
    let g1_x = chip.assign(&mut layouter, self.g1_x)?;
    let g1_y = chip.assign(&mut layouter, self.g1_y)?;
    let accumulator = AssignedMillerAccumulator::one(&chip, &mut layouter)?;
    let updated = accumulator.mul_by_line(&chip, &mut layouter, &line, &g1_x, &g1_y)?;
    let expected = AssignedFp12::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    updated.value.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}
