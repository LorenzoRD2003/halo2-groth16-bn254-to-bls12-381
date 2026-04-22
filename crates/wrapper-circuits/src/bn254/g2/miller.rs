use ff::Field;
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{
  AssignedFp, AssignedFp2, AssignedFp6, AssignedFp12, AssignedG2Affine, Bn254FieldChip,
  Bn254FieldConfig, ForeignField, Fp2Value, G2AffineConstant, G2AffineValue, G2LineCoeffsConstant,
  G2LineCoeffsValue, G2MillerPointConstant, NativeField, fp12_mul_constant, fp12_one_constant,
  fp12_square_constant, g1_generator_constant, g2_affine_from_miller_point_constant,
  g2_curve_coeff_b, g2_generator, g2_line_evaluation_constant, g2_miller_double_with_line_constant,
  g2_miller_mixed_add_with_line_constant, g2_miller_point_from_affine_constant,
};
use super::{Fp12Constant, Fp12Value};

type MillerAccumulatorFixed = (
  ((ForeignField, ForeignField), (ForeignField, ForeignField), (ForeignField, ForeignField)),
  ((ForeignField, ForeignField), (ForeignField, ForeignField), (ForeignField, ForeignField)),
);

/// Narrow affine G1 evaluation point for Miller-line consumption.
///
/// The current Miller slice needs affine G1 coordinates for sparse line
/// evaluation, while the broader repository still exposes opaque Midnight G1
/// points through `AssignedG1`. This dedicated coordinate pair keeps the
/// accumulation boundary small and explicit until a wider pairing-ready G1
/// surface is planned.
#[derive(Clone, Debug)]
pub struct AssignedG1Point {
  /// Affine x-coordinate.
  pub x: AssignedFp,
  /// Affine y-coordinate.
  pub y: AssignedFp,
}

impl AssignedG1Point {
  /// Builds a G1 evaluation point from assigned affine coordinates.
  #[must_use]
  pub fn new(x: AssignedFp, y: AssignedFp) -> Self {
    Self { x, y }
  }

  /// Assigns a G1 evaluation point from affine coordinates.
  ///
  /// This helper intentionally does not widen the repo's current G1 API. The
  /// caller is expected to supply coordinates from a valid affine BN254 G1
  /// point, typically sourced from existing host/reference helpers.
  ///
  /// # Errors
  ///
  /// Returns an error if either underlying coordinate assignment fails.
  pub fn assign(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    x: Value<ForeignField>,
    y: Value<ForeignField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(chip.assign(layouter, x)?, chip.assign(layouter, y)?))
  }
}

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

  /// Evaluates this sparse BN254 D-twist line at an affine G1 point.
  ///
  /// The sparse embedding stays owned by the Miller layer rather than becoming
  /// a broad `AssignedFp12` helper. This keeps later `mul_by_034`-style
  /// specialization localized to the accumulator boundary.
  ///
  /// # Errors
  ///
  /// Returns an error if any underlying Fp/Fp2 assignment or multiplication
  /// fails.
  pub fn evaluate_at_g1(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    point: &AssignedG1Point,
  ) -> Result<AssignedFp12, Error> {
    let slot_c0 = self.ell_0.scale_by_fp(chip, layouter, &point.y)?;
    let slot_c3 = self.ell_w.scale_by_fp(chip, layouter, &point.x)?;
    let zero_fp2 = AssignedFp2::zero(chip, layouter)?;

    Ok(AssignedFp12::new(
      AssignedFp6::new(slot_c0, zero_fp2.clone(), zero_fp2.clone()),
      AssignedFp6::new(slot_c3, self.ell_vw.clone(), zero_fp2),
    ))
  }
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
  /// Current Miller accumulator value.
  pub f: AssignedFp12,
}

impl AssignedMillerAccumulator {
  /// Builds an accumulator from an assigned Fp12 value.
  #[must_use]
  pub fn new(f: AssignedFp12) -> Self {
    Self { f }
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

  /// Squares the current accumulator value.
  ///
  /// Constraint cost is concentrated in the underlying Fp12 square. The Miller
  /// layer keeps the sequencing logic here so future loop optimizations do not
  /// leak into the general Fp12 API.
  ///
  /// # Errors
  ///
  /// Returns an error if the underlying Fp12 square fails.
  pub fn square(
    &mut self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    self.f = self.f.square(chip, layouter)?;
    Ok(())
  }

  /// Multiplies the accumulator by an already-evaluated Fp12 line value.
  ///
  /// This is where a future `mul_by_034`-style sparse specialization should
  /// plug in. The first narrow slice keeps the public boundary stable while
  /// honestly using a full Fp12 multiply internally.
  ///
  /// # Errors
  ///
  /// Returns an error if the underlying Fp12 multiplication fails.
  pub fn mul_by_evaluated_line(
    &mut self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp12,
  ) -> Result<(), Error> {
    self.f = self.f.mul(chip, layouter, value)?;
    Ok(())
  }

  fn mul_by_line_evaluated_generic(
    &mut self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    line: &AssignedG2LineCoeffs,
    point: &AssignedG1Point,
  ) -> Result<(), Error> {
    let line_value = line.evaluate_at_g1(chip, layouter, point)?;
    self.mul_by_evaluated_line(chip, layouter, &line_value)
  }

  fn mul_by_line_evaluated_sparse(
    &mut self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    line: &AssignedG2LineCoeffs,
    point: &AssignedG1Point,
  ) -> Result<(), Error> {
    let c0 = line.ell_0.scale_by_fp(chip, layouter, &point.y)?;
    let c3 = line.ell_w.scale_by_fp(chip, layouter, &point.x)?;
    let c4 = line.ell_vw.clone();

    // This is the BN254 D-twist `mul_by_034` path specialized to our
    // `(c0, c3, c4)` sparse embedding. The heavy cost reduction comes from
    // avoiding a generic Fp12 materialization followed by a near-full Fp12 mul.
    let a = self.f.c0.scale_by_fp2(chip, layouter, &c0)?;
    let b = self.f.c1.mul_by_01(chip, layouter, &c3, &c4)?;
    let c0_plus_c3 = c0.add(chip, layouter, &c3)?;
    let c =
      self.f.c0.add(chip, layouter, &self.f.c1)?.mul_by_01(chip, layouter, &c0_plus_c3, &c4)?;

    let next_c1 = c.sub(chip, layouter, &a)?.sub(chip, layouter, &b)?;
    let b_nr = b.mul_by_nonresidue(chip, layouter)?;
    let next_c0 = a.add(chip, layouter, &b_nr)?;
    self.f = AssignedFp12::new(next_c0, next_c1);
    Ok(())
  }

  /// Multiplies the accumulator by the sparse evaluation of a G2 line at a G1
  /// affine point.
  ///
  /// Sparse line evaluation stays inside the accumulator boundary so later
  /// sparse-specialized multiplication can replace the generic Fp12 multiply
  /// without changing the public API.
  ///
  /// # Errors
  ///
  /// Returns an error if the sparse line evaluation or Fp12 multiplication fails.
  pub fn mul_by_line(
    &mut self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    line: &AssignedG2LineCoeffs,
    point: &AssignedG1Point,
  ) -> Result<(), Error> {
    self.mul_by_line_evaluated_sparse(chip, layouter, line, point)
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
    self.f.assert_equal_to_fixed(chip, layouter, expected)
  }
}

/// One fixed Miller-loop schedule step.
#[derive(Clone, Debug)]
pub enum MillerStep {
  /// Doubling line. The driver squares before consuming this line.
  Double {
    /// Extracted sparse line coefficients for this doubling step.
    line: AssignedG2LineCoeffs,
  },
  /// Mixed-add line. The driver consumes it without an extra square.
  Add {
    /// Extracted sparse line coefficients for this mixed-add step.
    line: AssignedG2LineCoeffs,
  },
}

impl MillerStep {
  fn line(&self) -> &AssignedG2LineCoeffs {
    match self {
      Self::Double { line } | Self::Add { line } => line,
    }
  }

  fn requires_square(&self) -> bool {
    matches!(self, Self::Double { .. })
  }
}

/// Prepared Miller schedule with an explicit fixed host-side traversal order.
#[derive(Clone, Debug, Default)]
pub struct PreparedG2Miller {
  /// Expanded Miller traversal steps.
  pub steps: Vec<MillerStep>,
}

impl PreparedG2Miller {
  /// Builds a prepared Miller schedule from explicit steps.
  #[must_use]
  pub fn new(steps: Vec<MillerStep>) -> Self {
    Self { steps }
  }
}

/// Runs the narrow Miller accumulation slice over a fixed prepared schedule.
///
/// This is intentionally only the accumulation over extracted line
/// coefficients. It does not implement final exponentiation or claim to be a
/// full pairing API.
///
/// Host-side schedule branching is explicit and deterministic: witness values
/// never decide whether a square or add occurs.
///
/// # Errors
///
/// Returns an error if any underlying accumulator step fails.
pub fn miller_loop(
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  point: &AssignedG1Point,
  prepared: &PreparedG2Miller,
) -> Result<AssignedFp12, Error> {
  let mut accumulator = AssignedMillerAccumulator::one(chip, layouter)?;

  for step in &prepared.steps {
    if step.requires_square() {
      accumulator.square(chip, layouter)?;
    }
    accumulator.mul_by_line(chip, layouter, step.line(), point)?;
  }

  Ok(accumulator.f)
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

/// Small circuit that exercises a Miller-accumulator square.
#[derive(Clone, Debug)]
pub struct MillerAccumulatorSquareCircuit {
  input: Fp12Value,
  expected: Fp12Value,
}

impl MillerAccumulatorSquareCircuit {
  /// Builds a square circuit from an input and expected Fp12 output.
  #[must_use]
  pub fn new(input: &Fp12Constant, expected: &Fp12Constant) -> Self {
    Self {
      input: (
        (
          (Value::known(input.0.0.0), Value::known(input.0.0.1)),
          (Value::known(input.0.1.0), Value::known(input.0.1.1)),
          (Value::known(input.0.2.0), Value::known(input.0.2.1)),
        ),
        (
          (Value::known(input.1.0.0), Value::known(input.1.0.1)),
          (Value::known(input.1.1.0), Value::known(input.1.1.1)),
          (Value::known(input.1.2.0), Value::known(input.1.2.1)),
        ),
      ),
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

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let g1 = g1_generator_constant();
    let generator = g2_generator();
    let (_, line) =
      g2_miller_double_with_line_constant(g2_miller_point_from_affine_constant(generator));
    let input = g2_line_evaluation_constant(line, g1);
    let expected = fp12_square_constant(&input);

    Self::new(&input, &expected)
  }
}

impl Default for MillerAccumulatorSquareCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for MillerAccumulatorSquareCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      input: (
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
    let input = AssignedFp12::assign(&chip, &mut layouter, self.input.0, self.input.1)?;
    let expected = AssignedFp12::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    let mut accumulator = AssignedMillerAccumulator::new(input);
    accumulator.square(&chip, &mut layouter)?;
    accumulator.f.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises Miller-accumulator multiplication by one sparse G2 line.
#[derive(Clone, Debug)]
pub struct MillerAccumulatorMulByLineCircuit {
  line: G2LineCoeffsValue,
  g1: (Value<ForeignField>, Value<ForeignField>),
  expected: Fp12Value,
}

impl MillerAccumulatorMulByLineCircuit {
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
      g1: (Value::known(g1_x), Value::known(g1_y)),
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

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let g1 = g1_generator_constant();
    let generator = g2_generator();
    let (_, line) =
      g2_miller_double_with_line_constant(g2_miller_point_from_affine_constant(generator));
    let expected = g2_line_evaluation_constant(line, g1);

    Self::new(line, g1.0, g1.1, &expected)
  }
}

impl Default for MillerAccumulatorMulByLineCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for MillerAccumulatorMulByLineCircuit {
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
      g1: (Value::unknown(), Value::unknown()),
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
    let point = AssignedG1Point::assign(&chip, &mut layouter, self.g1.0, self.g1.1)?;
    let mut accumulator = AssignedMillerAccumulator::one(&chip, &mut layouter)?;
    accumulator.mul_by_line_evaluated_generic(&chip, &mut layouter, &line, &point)?;
    let expected = AssignedFp12::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    accumulator.f.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises the optimized sparse Miller mul-by-line path.
#[derive(Clone, Debug)]
pub struct MillerAccumulatorMulByLineSparseCircuit {
  line: G2LineCoeffsValue,
  g1: (Value<ForeignField>, Value<ForeignField>),
  expected: Fp12Value,
}

impl MillerAccumulatorMulByLineSparseCircuit {
  /// Builds a new optimized Miller mul-by-line circuit with a known Fp12 output.
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
      g1: (Value::known(g1_x), Value::known(g1_y)),
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

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let g1 = g1_generator_constant();
    let generator = g2_generator();
    let (_, line) =
      g2_miller_double_with_line_constant(g2_miller_point_from_affine_constant(generator));
    let expected = g2_line_evaluation_constant(line, g1);

    Self::new(line, g1.0, g1.1, &expected)
  }
}

impl Default for MillerAccumulatorMulByLineSparseCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for MillerAccumulatorMulByLineSparseCircuit {
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
      g1: (Value::unknown(), Value::unknown()),
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
    let point = AssignedG1Point::assign(&chip, &mut layouter, self.g1.0, self.g1.1)?;
    let mut accumulator = AssignedMillerAccumulator::one(&chip, &mut layouter)?;
    accumulator.mul_by_line(&chip, &mut layouter, &line, &point)?;
    let expected = AssignedFp12::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    accumulator.f.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}

#[derive(Clone, Debug)]
enum MillerStepValue {
  Double(G2LineCoeffsValue),
  Add(G2LineCoeffsValue),
}

/// Fixed host-side Miller step constants used by sample circuits and tests.
#[derive(Clone, Debug)]
pub enum MillerStepConstant {
  /// Doubling line encoded as fixed host-side coefficients.
  Double(G2LineCoeffsConstant),
  /// Mixed-add line encoded as fixed host-side coefficients.
  Add(G2LineCoeffsConstant),
}

impl MillerStepValue {
  fn without_witnesses(&self) -> Self {
    match self {
      Self::Double(_) => Self::Double((
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
      )),
      Self::Add(_) => Self::Add((
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
        (Value::unknown(), Value::unknown()),
      )),
    }
  }
}

/// Small circuit that exercises the narrow Miller-loop accumulation driver.
#[derive(Clone, Debug)]
pub struct MillerLoopCircuit {
  g1: (Value<ForeignField>, Value<ForeignField>),
  steps: Vec<MillerStepValue>,
  expected: Fp12Value,
}

impl MillerLoopCircuit {
  /// Builds a circuit from a prepared Miller schedule and expected Fp12 output.
  #[must_use]
  pub fn new(
    g1: (ForeignField, ForeignField),
    steps: Vec<MillerStepConstant>,
    expected: &Fp12Constant,
  ) -> Self {
    let steps = steps
      .into_iter()
      .map(|step| match step {
        MillerStepConstant::Double(line) => MillerStepValue::Double((
          (Value::known(line.0.0), Value::known(line.0.1)),
          (Value::known(line.1.0), Value::known(line.1.1)),
          (Value::known(line.2.0), Value::known(line.2.1)),
        )),
        MillerStepConstant::Add(line) => MillerStepValue::Add((
          (Value::known(line.0.0), Value::known(line.0.1)),
          (Value::known(line.1.0), Value::known(line.1.1)),
          (Value::known(line.2.0), Value::known(line.2.1)),
        )),
      })
      .collect();

    Self {
      g1: (Value::known(g1.0), Value::known(g1.1)),
      steps,
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

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let g1 = g1_generator_constant();
    let generator = g2_generator();
    let (doubled_state, double_line) =
      g2_miller_double_with_line_constant(g2_miller_point_from_affine_constant(generator));
    let (_, add_line) = g2_miller_mixed_add_with_line_constant(doubled_state, generator);
    let expected = fp12_mul_constant(
      &fp12_mul_constant(
        &fp12_square_constant(&fp12_one_constant()),
        &g2_line_evaluation_constant(double_line, g1),
      ),
      &g2_line_evaluation_constant(add_line, g1),
    );

    Self {
      g1: (Value::known(g1.0), Value::known(g1.1)),
      steps: vec![
        MillerStepValue::Double((
          (Value::known(double_line.0.0), Value::known(double_line.0.1)),
          (Value::known(double_line.1.0), Value::known(double_line.1.1)),
          (Value::known(double_line.2.0), Value::known(double_line.2.1)),
        )),
        MillerStepValue::Add((
          (Value::known(add_line.0.0), Value::known(add_line.0.1)),
          (Value::known(add_line.1.0), Value::known(add_line.1.1)),
          (Value::known(add_line.2.0), Value::known(add_line.2.1)),
        )),
      ],
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

impl Default for MillerLoopCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for MillerLoopCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      g1: (Value::unknown(), Value::unknown()),
      steps: self.steps.iter().map(MillerStepValue::without_witnesses).collect(),
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
    let point = AssignedG1Point::assign(&chip, &mut layouter, self.g1.0, self.g1.1)?;
    let mut prepared = Vec::with_capacity(self.steps.len());

    for step in &self.steps {
      let line = match step {
        MillerStepValue::Double(line) | MillerStepValue::Add(line) => {
          AssignedG2LineCoeffs::assign(&chip, &mut layouter, line.0, line.1, line.2)?
        }
      };
      prepared.push(match step {
        MillerStepValue::Double(_) => MillerStep::Double { line },
        MillerStepValue::Add(_) => MillerStep::Add { line },
      });
    }

    let expected = AssignedFp12::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    let actual = miller_loop(&chip, &mut layouter, &point, &PreparedG2Miller::new(prepared))?;
    actual.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}
