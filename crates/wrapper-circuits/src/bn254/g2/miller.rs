use ff::{Field, PrimeField};
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{
  AssignedFp, AssignedFp2, AssignedFp6, AssignedFp12, AssignedG2Affine, Bn254FieldChip,
  Bn254FieldConfig, ForeignField, Fp2Value, G2AffineConstant, G2AffineValue, G2LineCoeffsConstant,
  G2LineCoeffsValue, G2MillerPointConstant, NativeField, bn254_final_exponentiation_constant,
  fp12_mul_constant, fp12_one_constant, fp12_square_constant, g1_generator_constant,
  g2_affine_from_miller_point_constant, g2_curve_coeff_b, g2_generator,
  g2_line_evaluation_constant, g2_miller_double_with_line_constant,
  g2_miller_mixed_add_with_line_constant, g2_miller_point_from_affine_constant,
};
use super::{Fp12Constant, Fp12Value};
use crate::bn254::host::{
  Fp2Constant, bn254_final_exponentiation_easy_part_constant,
  bn254_final_exponentiation_hard_part_constant, fp2_mul_constant, fp2_neg_constant,
};
use crate::bn254::{AssignedBool, Bn254BoolChip, Bn254BoolConfig};

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

/// Host-side addend source for the fixed BN254 optimal-ate Miller schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bn254MillerAddend {
  /// Add the original affine `Q`.
  Base,
  /// Add `-Q` for a `-1` loop-count digit.
  NegBase,
  /// Add the first Frobenius image `q1 = [p]Q`.
  FrobeniusQ1,
  /// Add the second Frobenius-tail point used by arkworks after negating its `y` coordinate.
  FrobeniusQ2NegY,
}

/// One host-side step in the fixed BN254 optimal-ate Miller traversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bn254MillerScheduleStep {
  /// Square the accumulator, then consume the doubling line.
  Double,
  /// Consume the mixed-add line for the selected fixed addend.
  Add(Bn254MillerAddend),
}

/// Host-side fixed BN254 Miller schedule derived from the standard optimal-ate loop count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bn254MillerSchedule {
  /// Expanded deterministic traversal order.
  pub steps: Vec<Bn254MillerScheduleStep>,
}

impl Bn254MillerSchedule {
  /// Returns the fixed BN254 optimal-ate traversal used by arkworks prepared-G2 generation.
  #[must_use]
  pub fn bn254() -> Self {
    let mut steps = Vec::with_capacity(2 * bn254_ate_loop_count().len());

    for digit in bn254_ate_loop_count().iter().rev().skip(1) {
      steps.push(Bn254MillerScheduleStep::Double);

      match digit {
        1 => steps.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::Base)),
        -1 => steps.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::NegBase)),
        0 => {}
        _ => unreachable!("BN254 optimal-ate loop digits are ternary in {{-1, 0, 1}}"),
      }
    }

    steps.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::FrobeniusQ1));
    steps.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::FrobeniusQ2NegY));

    Self { steps }
  }
}

/// Returns the fixed BN254 optimal-ate Miller loop digits as used by arkworks.
#[must_use]
pub fn bn254_ate_loop_count() -> &'static [i8] {
  &[
    0, 0, 0, 1, 0, 1, 0, -1, 0, 0, -1, 0, 0, 0, 1, 0, 0, -1, 0, -1, 0, 0, 0, 1, 0, -1, 0, 0, 0, 0,
    -1, 0, 0, 1, 0, -1, 0, 0, 1, 0, 0, 0, 0, 0, -1, 0, 0, -1, 0, 1, 0, -1, 0, 0, 0, -1, 0, -1, 0,
    0, 0, 1, 0, 1, 1,
  ]
}

fn fp2_frobenius_map_constant(value: Fp2Constant) -> Fp2Constant {
  (value.0, -value.1)
}

fn assign_fp2_constant(
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  value: Fp2Constant,
) -> Result<AssignedFp2, Error> {
  AssignedFp2::assign(chip, layouter, Value::known(value.0), Value::known(value.1))
}

fn fp2_frobenius_map(
  value: &AssignedFp2,
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
) -> Result<AssignedFp2, Error> {
  Ok(AssignedFp2::new(value.c0.clone(), chip.neg(layouter, &value.c1)?))
}

fn g2_mul_by_char(
  point: &AssignedG2Affine,
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
) -> Result<AssignedG2Affine, Error> {
  let twist_mul_by_q_x = assign_fp2_constant(
    chip,
    layouter,
    (
      ForeignField::from_str_vartime(
        "21575463638280843010398324269430826099269044274347216827212613867836435027261",
      )
      .expect("hard-coded BN254 twist mul-by-q x.c0 should parse"),
      ForeignField::from_str_vartime(
        "10307601595873709700152284273816112264069230130616436755625194854815875713954",
      )
      .expect("hard-coded BN254 twist mul-by-q x.c1 should parse"),
    ),
  )?;
  let twist_mul_by_q_y = assign_fp2_constant(
    chip,
    layouter,
    (
      ForeignField::from_str_vartime(
        "2821565182194536844548159561693502659359617185244120367078079554186484126554",
      )
      .expect("hard-coded BN254 twist mul-by-q y.c0 should parse"),
      ForeignField::from_str_vartime(
        "3505843767911556378687030309984248845540243509899259641013678093033130930403",
      )
      .expect("hard-coded BN254 twist mul-by-q y.c1 should parse"),
    ),
  )?;

  let x = fp2_frobenius_map(&point.x, chip, layouter)?.mul(chip, layouter, &twist_mul_by_q_x)?;
  let y = fp2_frobenius_map(&point.y, chip, layouter)?.mul(chip, layouter, &twist_mul_by_q_y)?;
  Ok(AssignedG2Affine::new(x, y))
}

fn g2_mul_by_char_constant(point: G2AffineConstant) -> G2AffineConstant {
  let frobenius_x = fp2_frobenius_map_constant(point.0);
  let frobenius_y = fp2_frobenius_map_constant(point.1);
  let twist_mul_by_q_x = (
    ForeignField::from_str_vartime(
      "21575463638280843010398324269430826099269044274347216827212613867836435027261",
    )
    .expect("hard-coded BN254 twist mul-by-q x.c0 should parse"),
    ForeignField::from_str_vartime(
      "10307601595873709700152284273816112264069230130616436755625194854815875713954",
    )
    .expect("hard-coded BN254 twist mul-by-q x.c1 should parse"),
  );
  let twist_mul_by_q_y = (
    ForeignField::from_str_vartime(
      "2821565182194536844548159561693502659359617185244120367078079554186484126554",
    )
    .expect("hard-coded BN254 twist mul-by-q y.c0 should parse"),
    ForeignField::from_str_vartime(
      "3505843767911556378687030309984248845540243509899259641013678093033130930403",
    )
    .expect("hard-coded BN254 twist mul-by-q y.c1 should parse"),
  );

  (fp2_mul_constant(frobenius_x, twist_mul_by_q_x), fp2_mul_constant(frobenius_y, twist_mul_by_q_y))
}

fn bn254_prepared_miller_steps_constant(point: G2AffineConstant) -> Vec<MillerStepConstant> {
  let schedule = Bn254MillerSchedule::bn254();
  let neg_point = (point.0, fp2_neg_constant(point.1));
  let q1 = g2_mul_by_char_constant(point);
  let mut q2 = g2_mul_by_char_constant(q1);
  q2.1 = fp2_neg_constant(q2.1);

  let mut current = g2_miller_point_from_affine_constant(point);
  let mut prepared = Vec::with_capacity(schedule.steps.len());

  for step in schedule.steps {
    match step {
      Bn254MillerScheduleStep::Double => {
        let (next_point, line) = g2_miller_double_with_line_constant(current);
        current = next_point;
        prepared.push(MillerStepConstant::Double(line));
      }
      Bn254MillerScheduleStep::Add(addend) => {
        let selected_addend = match addend {
          Bn254MillerAddend::Base => point,
          Bn254MillerAddend::NegBase => neg_point,
          Bn254MillerAddend::FrobeniusQ1 => q1,
          Bn254MillerAddend::FrobeniusQ2NegY => q2,
        };
        let (next_point, line) = g2_miller_mixed_add_with_line_constant(current, selected_addend);
        current = next_point;
        prepared.push(MillerStepConstant::Add(line));
      }
    }
  }

  prepared
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

/// Host-side prepared Miller schedule for a fixed affine G2 point.
///
/// This is the constant-term companion to [`PreparedG2Miller`]. It stores the
/// exact per-step line-coefficient sequence for the fixed BN254 Miller schedule
/// so the circuit can consume those lines directly without performing G2
/// doubling / mixed-addition on constant verifier-key terms.
#[derive(Clone, Debug, Default)]
pub struct PreparedConstantG2Miller {
  /// Expanded Miller traversal steps, aligned with [`Bn254MillerSchedule::bn254()`].
  pub steps: Vec<MillerStepConstant>,
}

impl PreparedConstantG2Miller {
  /// Builds prepared constant Miller data from explicit schedule steps.
  #[must_use]
  pub fn new(steps: Vec<MillerStepConstant>) -> Self {
    Self { steps }
  }

  /// Prepares a fixed affine BN254 G2 point off-circuit.
  #[must_use]
  pub fn from_affine_constant(point: G2AffineConstant) -> Self {
    Self::new(bn254_prepared_miller_steps_constant(point))
  }

  fn validate_against_schedule(&self) -> Result<(), Error> {
    let schedule = Bn254MillerSchedule::bn254();
    if self.steps.len() != schedule.steps.len() {
      return Err(Error::Synthesis(format!(
        "prepared G2 schedule length mismatch: expected {}, got {}",
        schedule.steps.len(),
        self.steps.len()
      )));
    }

    for (index, (expected_step, prepared_step)) in
      schedule.steps.iter().zip(self.steps.iter()).enumerate()
    {
      let kinds_match = matches!(
        (expected_step, prepared_step),
        (Bn254MillerScheduleStep::Double, MillerStepConstant::Double(_))
          | (Bn254MillerScheduleStep::Add(_), MillerStepConstant::Add(_))
      );

      if !kinds_match {
        return Err(Error::Synthesis(format!(
          "prepared G2 schedule kind mismatch at step {}",
          index
        )));
      }
    }

    Ok(())
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

#[derive(Clone, Debug)]
struct AssignedVariableMultiMillerTerm {
  point: AssignedG1Point,
  current: AssignedG2MillerPoint,
  base: AssignedG2Affine,
  neg_base: AssignedG2Affine,
  frobenius_q1: AssignedG2Affine,
  frobenius_q2_neg_y: AssignedG2Affine,
}

impl AssignedVariableMultiMillerTerm {
  fn initialize(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    point: &AssignedG1Point,
    g2: &AssignedG2Affine,
  ) -> Result<Self, Error> {
    let neg_base = g2.neg(chip, layouter)?;
    let frobenius_q1 = g2_mul_by_char(g2, chip, layouter)?;
    let mut frobenius_q2_neg_y = g2_mul_by_char(&frobenius_q1, chip, layouter)?;
    frobenius_q2_neg_y =
      AssignedG2Affine::new(frobenius_q2_neg_y.x, frobenius_q2_neg_y.y.neg(chip, layouter)?);

    Ok(Self {
      point: point.clone(),
      current: AssignedG2MillerPoint::from_affine(g2, chip, layouter)?,
      base: g2.clone(),
      neg_base,
      frobenius_q1,
      frobenius_q2_neg_y,
    })
  }

  fn advance_step(
    &mut self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    step: Bn254MillerScheduleStep,
  ) -> Result<AssignedG2LineCoeffs, Error> {
    match step {
      Bn254MillerScheduleStep::Double => {
        let (next, line) = self.current.double_with_line(chip, layouter)?;
        self.current = next;
        Ok(line)
      }
      Bn254MillerScheduleStep::Add(addend) => {
        let selected = match addend {
          Bn254MillerAddend::Base => &self.base,
          Bn254MillerAddend::NegBase => &self.neg_base,
          Bn254MillerAddend::FrobeniusQ1 => &self.frobenius_q1,
          Bn254MillerAddend::FrobeniusQ2NegY => &self.frobenius_q2_neg_y,
        };
        let (next, line) = self.current.mixed_add_with_line(chip, layouter, selected)?;
        self.current = next;
        Ok(line)
      }
    }
  }
}

#[derive(Clone, Debug)]
struct AssignedPreparedConstantMultiMillerTerm {
  point: AssignedG1Point,
  prepared: PreparedConstantG2Miller,
  next_step: usize,
}

impl AssignedPreparedConstantMultiMillerTerm {
  fn initialize(
    point: &AssignedG1Point,
    prepared: &PreparedConstantG2Miller,
  ) -> Result<Self, Error> {
    prepared.validate_against_schedule()?;
    Ok(Self { point: point.clone(), prepared: prepared.clone(), next_step: 0 })
  }

  fn advance_step(
    &mut self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    step: Bn254MillerScheduleStep,
  ) -> Result<AssignedG2LineCoeffs, Error> {
    let prepared_step = self.prepared.steps.get(self.next_step).ok_or_else(|| {
      Error::Synthesis(format!("prepared G2 line sequence exhausted at step {}", self.next_step))
    })?;
    self.next_step += 1;

    let line = match (step, prepared_step) {
      (Bn254MillerScheduleStep::Double, MillerStepConstant::Double(line))
      | (Bn254MillerScheduleStep::Add(_), MillerStepConstant::Add(line)) => line,
      _ => {
        return Err(Error::Synthesis(format!(
          "prepared G2 line kind mismatch at step {}",
          self.next_step - 1
        )));
      }
    };

    AssignedG2LineCoeffs::assign(
      chip,
      layouter,
      (Value::known(line.0.0), Value::known(line.0.1)),
      (Value::known(line.1.0), Value::known(line.1.1)),
      (Value::known(line.2.0), Value::known(line.2.1)),
    )
  }
}

#[derive(Clone, Debug)]
enum AssignedInterleavedMultiMillerTerm {
  Variable(AssignedVariableMultiMillerTerm),
  Prepared(AssignedPreparedConstantMultiMillerTerm),
}

impl AssignedInterleavedMultiMillerTerm {
  fn advance_step(
    &mut self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    step: Bn254MillerScheduleStep,
  ) -> Result<AssignedG2LineCoeffs, Error> {
    match self {
      Self::Variable(term) => term.advance_step(chip, layouter, step),
      Self::Prepared(term) => term.advance_step(chip, layouter, step),
    }
  }

  fn point(&self) -> &AssignedG1Point {
    match self {
      Self::Variable(term) => &term.point,
      Self::Prepared(term) => &term.point,
    }
  }
}

fn run_interleaved_multi_miller_schedule(
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  terms: &mut [AssignedInterleavedMultiMillerTerm],
) -> Result<AssignedFp12, Error> {
  let mut accumulator = AssignedMillerAccumulator::one(chip, layouter)?;

  for step in &Bn254MillerSchedule::bn254().steps {
    if matches!(step, Bn254MillerScheduleStep::Double) {
      accumulator.square(chip, layouter)?;
    }

    for term in terms.iter_mut() {
      let line = term.advance_step(chip, layouter, *step)?;
      accumulator.mul_by_line(chip, layouter, &line, term.point())?;
    }
  }

  Ok(accumulator.f)
}

/// Computes the product of the real BN254 Miller-loop outputs for a list of terms.
///
/// This remains intentionally narrow and verifier-shaped: it reuses the fixed
/// real BN254 schedule for each term, multiplies the Miller outputs together,
/// and leaves the single shared final exponentiation to higher-level product
/// checks.
pub fn multi_miller_loop(
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  terms: &[(&AssignedG1Point, &AssignedG2Affine)],
) -> Result<AssignedFp12, Error> {
  if terms.is_empty() {
    return AssignedFp12::one(chip, layouter);
  }

  let mut initialized_terms = Vec::with_capacity(terms.len());
  for (g1, g2) in terms {
    initialized_terms.push(AssignedInterleavedMultiMillerTerm::Variable(
      AssignedVariableMultiMillerTerm::initialize(chip, layouter, g1, g2)?,
    ));
  }

  run_interleaved_multi_miller_schedule(chip, layouter, &mut initialized_terms)
}

pub fn multi_miller_loop_with_prepared_terms(
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  variable_terms: &[(&AssignedG1Point, &AssignedG2Affine)],
  prepared_terms: &[(&AssignedG1Point, &PreparedConstantG2Miller)],
) -> Result<AssignedFp12, Error> {
  if variable_terms.is_empty() && prepared_terms.is_empty() {
    return AssignedFp12::one(chip, layouter);
  }

  let mut initialized_terms = Vec::with_capacity(variable_terms.len() + prepared_terms.len());
  for (g1, g2) in variable_terms {
    initialized_terms.push(AssignedInterleavedMultiMillerTerm::Variable(
      AssignedVariableMultiMillerTerm::initialize(chip, layouter, g1, g2)?,
    ));
  }
  for (g1, prepared) in prepared_terms {
    initialized_terms.push(AssignedInterleavedMultiMillerTerm::Prepared(
      AssignedPreparedConstantMultiMillerTerm::initialize(g1, prepared)?,
    ));
  }

  run_interleaved_multi_miller_schedule(chip, layouter, &mut initialized_terms)
}

fn exp_by_neg_x(
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  value: &AssignedFp12,
) -> Result<AssignedFp12, Error> {
  use crate::bn254::{
    BN254_EXP_BY_X_CHAIN_START, BN254_EXP_BY_X_CHAIN_STEPS, BN254_X_ABS, Bn254ExpByXWindow,
  };

  fn cyclotomic_square_6_times(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp12,
  ) -> Result<AssignedFp12, Error> {
    let value = value.cyclotomic_square(chip, layouter)?;
    let value = value.cyclotomic_square(chip, layouter)?;
    let value = value.cyclotomic_square(chip, layouter)?;
    let value = value.cyclotomic_square(chip, layouter)?;
    let value = value.cyclotomic_square(chip, layouter)?;
    value.cyclotomic_square(chip, layouter)
  }

  fn cyclotomic_square_7_times(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp12,
  ) -> Result<AssignedFp12, Error> {
    let value = cyclotomic_square_6_times(chip, layouter, value)?;
    value.cyclotomic_square(chip, layouter)
  }

  fn cyclotomic_square_8_times(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp12,
  ) -> Result<AssignedFp12, Error> {
    let value = cyclotomic_square_7_times(chip, layouter, value)?;
    value.cyclotomic_square(chip, layouter)
  }

  fn cyclotomic_square_10_times(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp12,
  ) -> Result<AssignedFp12, Error> {
    let value = cyclotomic_square_8_times(chip, layouter, value)?;
    let value = value.cyclotomic_square(chip, layouter)?;
    value.cyclotomic_square(chip, layouter)
  }

  fn cyclotomic_square_n_times(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp12,
    square_count: u8,
  ) -> Result<AssignedFp12, Error> {
    match square_count {
      6 => cyclotomic_square_6_times(chip, layouter, value),
      7 => cyclotomic_square_7_times(chip, layouter, value),
      8 => cyclotomic_square_8_times(chip, layouter, value),
      10 => cyclotomic_square_10_times(chip, layouter, value),
      _ => unreachable!("unsupported BN254 exp-by-x square block"),
    }
  }

  fn exp_by_x_window<'a>(
    x17: &'a AssignedFp12,
    x25: &'a AssignedFp12,
    x29: &'a AssignedFp12,
    x39: &'a AssignedFp12,
    x41: &'a AssignedFp12,
    x43: &'a AssignedFp12,
    x49: &'a AssignedFp12,
    window: Bn254ExpByXWindow,
  ) -> &'a AssignedFp12 {
    match window {
      Bn254ExpByXWindow::X17 => x17,
      Bn254ExpByXWindow::X25 => x25,
      Bn254ExpByXWindow::X29 => x29,
      Bn254ExpByXWindow::X39 => x39,
      Bn254ExpByXWindow::X41 => x41,
      Bn254ExpByXWindow::X43 => x43,
      Bn254ExpByXWindow::X49 => x49,
    }
  }

  // Compute value^x for the BN254 parameter
  // x = 0x44e992b44a6909f1 = 4965661367192848881.
  //
  // The shift-and-add recipe itself lives in `bn254/final_exp_chain.rs` so the
  // host/reference path and the circuit path cannot silently diverge. Every
  // call in the hard part starts from a cyclotomic-subgroup element, so the
  // repeated square blocks below use cyclotomic_square(...) rather than the
  // generic Fp12 square(...).
  debug_assert_eq!(BN254_X_ABS, 0x44e9_92b4_4a69_09f1);
  let x2 = value.cyclotomic_square(chip, layouter)?;
  let x4 = x2.cyclotomic_square(chip, layouter)?;
  let x8 = x4.cyclotomic_square(chip, layouter)?;
  let x16 = x8.cyclotomic_square(chip, layouter)?;
  let x32 = x16.cyclotomic_square(chip, layouter)?;

  let x10 = x8.mul(chip, layouter, &x2)?;
  let x17 = x16.mul(chip, layouter, value)?;
  let x25 = x17.mul(chip, layouter, &x8)?;
  let x29 = x25.mul(chip, layouter, &x4)?;
  let x39 = x29.mul(chip, layouter, &x10)?;
  let x41 = x25.mul(chip, layouter, &x16)?;
  let x43 = x41.mul(chip, layouter, &x2)?;
  let x49 = x32.mul(chip, layouter, &x17)?;

  let mut exp =
    exp_by_x_window(&x17, &x25, &x29, &x39, &x41, &x43, &x49, BN254_EXP_BY_X_CHAIN_START).clone();

  for (square_count, window) in BN254_EXP_BY_X_CHAIN_STEPS {
    exp = cyclotomic_square_n_times(chip, layouter, &exp, *square_count)?;
    exp = exp.mul(
      chip,
      layouter,
      exp_by_x_window(&x17, &x25, &x29, &x39, &x41, &x43, &x49, *window),
    )?;
  }

  exp.unitary_inverse(chip, layouter)
}

fn final_exponentiation_easy_part(
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  value: &AssignedFp12,
) -> Result<AssignedFp12, Error> {
  let f1 = value.unitary_inverse(chip, layouter)?;
  let f2 = value.inv(chip, layouter)?;
  let mut r = f1.mul(chip, layouter, &f2)?;
  let r_clone = r.clone();
  r = r.frobenius_map(chip, layouter, 2)?;
  r.mul(chip, layouter, &r_clone)
}

fn final_exponentiation_hard_part(
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  value: &AssignedFp12,
) -> Result<AssignedFp12, Error> {
  // The easy part maps the Miller output into the cyclotomic subgroup, and the
  // hard-part multiplications and unitary inverses keep intermediates inside
  // that subgroup. The explicit square sites here can therefore use
  // cyclotomic_square(...).
  let r = value.clone();

  let y0 = exp_by_neg_x(chip, layouter, value)?;
  let y1 = y0.cyclotomic_square(chip, layouter)?;
  let y2 = y1.cyclotomic_square(chip, layouter)?;
  let mut y3 = y2.mul(chip, layouter, &y1)?;
  let y4 = exp_by_neg_x(chip, layouter, &y3)?;
  let y5 = y4.cyclotomic_square(chip, layouter)?;
  let mut y6 = exp_by_neg_x(chip, layouter, &y5)?;
  y3 = y3.unitary_inverse(chip, layouter)?;
  y6 = y6.unitary_inverse(chip, layouter)?;
  let y7 = y6.mul(chip, layouter, &y4)?;
  let mut y8 = y7.mul(chip, layouter, &y3)?;
  let y9 = y8.mul(chip, layouter, &y1)?;
  let y10 = y8.mul(chip, layouter, &y4)?;
  let y11 = y10.mul(chip, layouter, &r)?;
  let mut y12 = y9.frobenius_map(chip, layouter, 1)?;
  y12 = y12.mul(chip, layouter, &y11)?;
  y8 = y8.frobenius_map(chip, layouter, 2)?;
  let y14 = y8.mul(chip, layouter, &y12)?;
  let r_inv = r.unitary_inverse(chip, layouter)?;
  let mut y15 = r_inv.mul(chip, layouter, &y9)?;
  y15 = y15.frobenius_map(chip, layouter, 3)?;
  y15.mul(chip, layouter, &y14)
}

/// Runs the BN254 final exponentiation on a nonzero Miller-loop output.
///
/// This implements the standard easy-part / hard-part decomposition used by
/// arkworks for BN curves. The current slice is intentionally narrow: it
/// expects a nonzero Miller-loop output and does not widen into a full public
/// pairing API.
///
/// # Errors
///
/// Returns an error if any underlying Fp12 operation fails.
pub fn final_exponentiation(
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  value: &AssignedFp12,
) -> Result<AssignedFp12, Error> {
  let easy = final_exponentiation_easy_part(chip, layouter, value)?;
  final_exponentiation_hard_part(chip, layouter, &easy)
}

/// Checks whether a narrow BN254 multi-pairing product equals the target-group identity.
///
/// This computes each real Miller loop, multiplies the Miller outputs together,
/// applies exactly one final exponentiation to the total product, and returns a
/// native constrained boolean for the equality-to-one check.
pub fn pairing_check(
  chip: &Bn254FieldChip,
  bool_chip: &Bn254BoolChip,
  layouter: &mut impl Layouter<NativeField>,
  terms: &[(&AssignedG1Point, &AssignedG2Affine)],
) -> Result<AssignedBool, Error> {
  let total_miller = multi_miller_loop(chip, layouter, terms)?;
  let gt = final_exponentiation(chip, layouter, &total_miller)?;
  let c0_0 = chip.is_equal_to_fixed(layouter, &gt.c0.c0.c0, ForeignField::ONE)?;
  let c0_1 = chip.is_equal_to_fixed(layouter, &gt.c0.c0.c1, ForeignField::ZERO)?;
  let c0_2 = chip.is_equal_to_fixed(layouter, &gt.c0.c1.c0, ForeignField::ZERO)?;
  let c0_3 = chip.is_equal_to_fixed(layouter, &gt.c0.c1.c1, ForeignField::ZERO)?;
  let c0_4 = chip.is_equal_to_fixed(layouter, &gt.c0.c2.c0, ForeignField::ZERO)?;
  let c0_5 = chip.is_equal_to_fixed(layouter, &gt.c0.c2.c1, ForeignField::ZERO)?;
  let c1_0 = chip.is_equal_to_fixed(layouter, &gt.c1.c0.c0, ForeignField::ZERO)?;
  let c1_1 = chip.is_equal_to_fixed(layouter, &gt.c1.c0.c1, ForeignField::ZERO)?;
  let c1_2 = chip.is_equal_to_fixed(layouter, &gt.c1.c1.c0, ForeignField::ZERO)?;
  let c1_3 = chip.is_equal_to_fixed(layouter, &gt.c1.c1.c1, ForeignField::ZERO)?;
  let c1_4 = chip.is_equal_to_fixed(layouter, &gt.c1.c2.c0, ForeignField::ZERO)?;
  let c1_5 = chip.is_equal_to_fixed(layouter, &gt.c1.c2.c1, ForeignField::ZERO)?;

  bool_chip.and(layouter, &[c0_0, c0_1, c0_2, c0_3, c0_4, c0_5, c1_0, c1_1, c1_2, c1_3, c1_4, c1_5])
}

pub fn pairing_check_with_prepared_terms(
  chip: &Bn254FieldChip,
  bool_chip: &Bn254BoolChip,
  layouter: &mut impl Layouter<NativeField>,
  variable_terms: &[(&AssignedG1Point, &AssignedG2Affine)],
  prepared_terms: &[(&AssignedG1Point, &PreparedConstantG2Miller)],
) -> Result<AssignedBool, Error> {
  let total_miller =
    multi_miller_loop_with_prepared_terms(chip, layouter, variable_terms, prepared_terms)?;
  let gt = final_exponentiation(chip, layouter, &total_miller)?;
  let c0_0 = chip.is_equal_to_fixed(layouter, &gt.c0.c0.c0, ForeignField::ONE)?;
  let c0_1 = chip.is_equal_to_fixed(layouter, &gt.c0.c0.c1, ForeignField::ZERO)?;
  let c0_2 = chip.is_equal_to_fixed(layouter, &gt.c0.c1.c0, ForeignField::ZERO)?;
  let c0_3 = chip.is_equal_to_fixed(layouter, &gt.c0.c1.c1, ForeignField::ZERO)?;
  let c0_4 = chip.is_equal_to_fixed(layouter, &gt.c0.c2.c0, ForeignField::ZERO)?;
  let c0_5 = chip.is_equal_to_fixed(layouter, &gt.c0.c2.c1, ForeignField::ZERO)?;
  let c1_0 = chip.is_equal_to_fixed(layouter, &gt.c1.c0.c0, ForeignField::ZERO)?;
  let c1_1 = chip.is_equal_to_fixed(layouter, &gt.c1.c0.c1, ForeignField::ZERO)?;
  let c1_2 = chip.is_equal_to_fixed(layouter, &gt.c1.c1.c0, ForeignField::ZERO)?;
  let c1_3 = chip.is_equal_to_fixed(layouter, &gt.c1.c1.c1, ForeignField::ZERO)?;
  let c1_4 = chip.is_equal_to_fixed(layouter, &gt.c1.c2.c0, ForeignField::ZERO)?;
  let c1_5 = chip.is_equal_to_fixed(layouter, &gt.c1.c2.c1, ForeignField::ZERO)?;

  bool_chip.and(layouter, &[c0_0, c0_1, c0_2, c0_3, c0_4, c0_5, c1_0, c1_1, c1_2, c1_3, c1_4, c1_5])
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

fn miller_step_value_from_constant(step: MillerStepConstant) -> MillerStepValue {
  match step {
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
  }
}

fn bn254_miller_output_constant(
  g1: (ForeignField, ForeignField),
  g2: G2AffineConstant,
) -> Fp12Constant {
  bn254_prepared_miller_steps_constant(g2).into_iter().fold(
    fp12_one_constant(),
    |accumulator, step| match step {
      MillerStepConstant::Double(line) => fp12_mul_constant(
        &fp12_square_constant(&accumulator),
        &g2_line_evaluation_constant(line, g1),
      ),
      MillerStepConstant::Add(line) => {
        fp12_mul_constant(&accumulator, &g2_line_evaluation_constant(line, g1))
      }
    },
  )
}

/// Small circuit that exercises the narrow Miller-loop accumulation driver.
#[derive(Clone, Debug)]
pub struct MillerLoopCircuit {
  g1: (Value<ForeignField>, Value<ForeignField>),
  g2: G2AffineConstant,
  steps: Vec<MillerStepValue>,
  expected: Fp12Value,
}

impl MillerLoopCircuit {
  /// Builds a circuit for the real fixed BN254 optimal-ate Miller schedule.
  #[must_use]
  pub fn new(
    g1: (ForeignField, ForeignField),
    g2: G2AffineConstant,
    expected: &Fp12Constant,
  ) -> Self {
    let steps = bn254_prepared_miller_steps_constant(g2)
      .into_iter()
      .map(miller_step_value_from_constant)
      .collect();

    Self {
      g1: (Value::known(g1.0), Value::known(g1.1)),
      g2,
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
    let expected = bn254_miller_output_constant(g1, generator);

    Self {
      g1: (Value::known(g1.0), Value::known(g1.1)),
      g2: generator,
      steps: bn254_prepared_miller_steps_constant(generator)
        .into_iter()
        .map(miller_step_value_from_constant)
        .collect(),
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
      g2: self.g2,
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

#[derive(Clone, Debug)]
struct FixedFp12UnaryCircuitIo {
  input: Fp12Value,
  expected: Fp12Value,
}

impl FixedFp12UnaryCircuitIo {
  fn new(input: &Fp12Constant, expected: &Fp12Constant) -> Self {
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
}

fn synthesize_fixed_fp12_unary_circuit<L, Op>(
  io: &FixedFp12UnaryCircuitIo,
  config: Bn254FieldConfig,
  mut layouter: L,
  op: Op,
) -> Result<(), Error>
where
  L: midnight_proofs::circuit::Layouter<NativeField>,
  Op: FnOnce(&Bn254FieldChip, &mut L, &AssignedFp12) -> Result<AssignedFp12, Error>,
{
  let chip = Bn254FieldChip::new(&config);
  let input = AssignedFp12::assign(&chip, &mut layouter, io.input.0, io.input.1)?;
  let expected = AssignedFp12::assign(&chip, &mut layouter, io.expected.0, io.expected.1)?;
  let actual = op(&chip, &mut layouter, &input)?;
  actual.assert_equal(&chip, &mut layouter, &expected)?;
  chip.load(&mut layouter)
}

/// Small circuit that exercises BN254 final exponentiation on a fixed Fp12 input.
#[derive(Clone, Debug)]
pub struct FinalExponentiationCircuit {
  io: FixedFp12UnaryCircuitIo,
}

impl FinalExponentiationCircuit {
  /// Builds a final-exponentiation circuit from a fixed Fp12 input and expected output.
  #[must_use]
  pub fn new(input: &Fp12Constant, expected: &Fp12Constant) -> Self {
    Self { io: FixedFp12UnaryCircuitIo::new(input, expected) }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let g1 = g1_generator_constant();
    let g2 = g2_generator();
    let miller_input = bn254_miller_output_constant(g1, g2);
    let expected = bn254_final_exponentiation_constant(&miller_input);
    Self::new(&miller_input, &expected)
  }
}

/// Small circuit that exercises only the easy part of BN254 final exponentiation.
#[derive(Clone, Debug)]
pub struct FinalExponentiationEasyPartCircuit {
  io: FixedFp12UnaryCircuitIo,
}

impl FinalExponentiationEasyPartCircuit {
  /// Builds an easy-part circuit from a fixed Fp12 input and expected output.
  #[must_use]
  pub fn new(input: &Fp12Constant, expected: &Fp12Constant) -> Self {
    Self { io: FixedFp12UnaryCircuitIo::new(input, expected) }
  }

  /// Returns a deterministic sample circuit suitable for metrics and profiling.
  #[must_use]
  pub fn sample() -> Self {
    let g1 = g1_generator_constant();
    let g2 = g2_generator();
    let miller_input = bn254_miller_output_constant(g1, g2);
    let expected = bn254_final_exponentiation_easy_part_constant(&miller_input);
    Self::new(&miller_input, &expected)
  }
}

impl Default for FinalExponentiationEasyPartCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for FinalExponentiationEasyPartCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { io: self.io.without_witnesses() }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    synthesize_fixed_fp12_unary_circuit(&self.io, config, layouter, |chip, layouter, input| {
      final_exponentiation_easy_part(chip, layouter, input)
    })
  }
}

/// Small circuit that exercises only the hard part of BN254 final exponentiation.
#[derive(Clone, Debug)]
pub struct FinalExponentiationHardPartCircuit {
  io: FixedFp12UnaryCircuitIo,
}

impl FinalExponentiationHardPartCircuit {
  /// Builds a hard-part circuit from a fixed easy-part input and expected output.
  #[must_use]
  pub fn new(input: &Fp12Constant, expected: &Fp12Constant) -> Self {
    Self { io: FixedFp12UnaryCircuitIo::new(input, expected) }
  }

  /// Returns a deterministic sample circuit suitable for metrics and profiling.
  #[must_use]
  pub fn sample() -> Self {
    let g1 = g1_generator_constant();
    let g2 = g2_generator();
    let miller_input = bn254_miller_output_constant(g1, g2);
    let easy = bn254_final_exponentiation_easy_part_constant(&miller_input);
    let expected = bn254_final_exponentiation_hard_part_constant(&easy);
    Self::new(&easy, &expected)
  }
}

impl Default for FinalExponentiationHardPartCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for FinalExponentiationHardPartCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { io: self.io.without_witnesses() }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    synthesize_fixed_fp12_unary_circuit(&self.io, config, layouter, |chip, layouter, input| {
      final_exponentiation_hard_part(chip, layouter, input)
    })
  }
}

impl Default for FinalExponentiationCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for FinalExponentiationCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { io: self.io.without_witnesses() }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    synthesize_fixed_fp12_unary_circuit(&self.io, config, layouter, |chip, layouter, input| {
      final_exponentiation(chip, layouter, input)
    })
  }
}

/// Small circuit that exercises Miller loop plus final exponentiation end-to-end.
#[derive(Clone, Debug)]
pub struct PairingFinalExponentiationCircuit {
  g1: (Value<ForeignField>, Value<ForeignField>),
  g2: G2AffineConstant,
  expected: Fp12Value,
}

impl PairingFinalExponentiationCircuit {
  /// Builds an end-to-end Miller-plus-final-exp circuit from affine inputs and expected GT output.
  #[must_use]
  pub fn new(
    g1: (ForeignField, ForeignField),
    g2: G2AffineConstant,
    expected: &Fp12Constant,
  ) -> Self {
    Self {
      g1: (Value::known(g1.0), Value::known(g1.1)),
      g2,
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
    let g2 = g2_generator();
    let miller = bn254_miller_output_constant(g1, g2);
    let expected = bn254_final_exponentiation_constant(&miller);
    Self::new(g1, g2, &expected)
  }
}

type PairingTermValue = ((Value<ForeignField>, Value<ForeignField>), G2AffineValue);
type PreparedPairingTermValue =
  ((Value<ForeignField>, Value<ForeignField>), PreparedConstantG2Miller);

/// Small circuit that exercises the narrow BN254 multi-pairing product check.
#[derive(Clone, Debug)]
pub struct PairingCheckCircuit {
  variable_terms: Vec<PairingTermValue>,
  prepared_terms: Vec<PreparedPairingTermValue>,
  expected: bool,
}

#[derive(Clone, Debug)]
pub struct PairingCheckConfig {
  field: Bn254FieldConfig,
  bools: Bn254BoolConfig,
}

impl PairingCheckCircuit {
  /// Builds a pairing-product-check circuit from fixed affine terms and an expected boolean result.
  #[must_use]
  pub fn new(terms: &[((ForeignField, ForeignField), G2AffineConstant)], expected: bool) -> Self {
    Self {
      variable_terms: terms
        .iter()
        .map(|term| {
          (
            (Value::known((term.0).0), Value::known((term.0).1)),
            (
              (Value::known(((term.1).0).0), Value::known(((term.1).0).1)),
              (Value::known(((term.1).1).0), Value::known(((term.1).1).1)),
            ),
          )
        })
        .collect(),
      prepared_terms: Vec::new(),
      expected,
    }
  }

  /// Builds a pairing-product-check circuit with variable and prepared constant G2 terms.
  #[must_use]
  pub fn new_with_prepared_constant_terms(
    variable_terms: &[((ForeignField, ForeignField), G2AffineConstant)],
    prepared_terms: &[((ForeignField, ForeignField), PreparedConstantG2Miller)],
    expected: bool,
  ) -> Self {
    Self {
      variable_terms: variable_terms
        .iter()
        .map(|term| {
          (
            (Value::known((term.0).0), Value::known((term.0).1)),
            (
              (Value::known(((term.1).0).0), Value::known(((term.1).0).1)),
              (Value::known(((term.1).1).0), Value::known(((term.1).1).1)),
            ),
          )
        })
        .collect(),
      prepared_terms: prepared_terms
        .iter()
        .map(|term| ((Value::known((term.0).0), Value::known((term.0).1)), term.1.clone()))
        .collect(),
      expected,
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let g1 = g1_generator_constant();
    let neg_g1 = (g1.0, -g1.1);
    let g2 = g2_generator();
    Self::new(&[(g1, g2), (neg_g1, g2)], true)
  }
}

impl Default for PairingCheckCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for PairingCheckCircuit {
  type Config = PairingCheckConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      variable_terms: self
        .variable_terms
        .iter()
        .map(|_| {
          (
            (Value::unknown(), Value::unknown()),
            ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
          )
        })
        .collect(),
      prepared_terms: self
        .prepared_terms
        .iter()
        .map(|term| ((Value::unknown(), Value::unknown()), term.1.clone()))
        .collect(),
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    let instance_columns = [meta.instance_column(), meta.instance_column()];
    PairingCheckConfig {
      field: Bn254FieldConfig::configure_with_instances(meta, &instance_columns),
      bools: Bn254BoolConfig::configure_with_instances(meta, &instance_columns),
    }
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254FieldChip::new(&config.field);
    let bool_chip = Bn254BoolChip::new(&config.bools);
    let mut assigned_variable_terms = Vec::with_capacity(self.variable_terms.len());
    let mut assigned_prepared_terms = Vec::with_capacity(self.prepared_terms.len());

    for (g1, g2) in &self.variable_terms {
      let assigned_g1 = AssignedG1Point::assign(&chip, &mut layouter, g1.0, g1.1)?;
      let assigned_g2 = AssignedG2Affine::assign(&chip, &mut layouter, g2.0, g2.1)?;
      assigned_variable_terms.push((assigned_g1, assigned_g2));
    }

    for (g1, prepared) in &self.prepared_terms {
      let assigned_g1 = AssignedG1Point::assign(&chip, &mut layouter, g1.0, g1.1)?;
      assigned_prepared_terms.push((assigned_g1, prepared.clone()));
    }

    let borrowed_variable_terms: Vec<_> =
      assigned_variable_terms.iter().map(|term| (&term.0, &term.1)).collect();
    let borrowed_prepared_terms: Vec<_> =
      assigned_prepared_terms.iter().map(|term| (&term.0, &term.1)).collect();
    let result = pairing_check_with_prepared_terms(
      &chip,
      &bool_chip,
      &mut layouter,
      &borrowed_variable_terms,
      &borrowed_prepared_terms,
    )?;
    bool_chip.assert_equal_to_fixed(&mut layouter, &result, self.expected)?;
    chip.load(&mut layouter)?;
    bool_chip.load(&mut layouter)
  }
}

impl Default for PairingFinalExponentiationCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for PairingFinalExponentiationCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      g1: (Value::unknown(), Value::unknown()),
      g2: self.g2,
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
    let mut prepared = Vec::with_capacity(bn254_prepared_miller_steps_constant(self.g2).len());

    for step in bn254_prepared_miller_steps_constant(self.g2) {
      let line = match step {
        MillerStepConstant::Double(line) | MillerStepConstant::Add(line) => {
          AssignedG2LineCoeffs::assign(
            &chip,
            &mut layouter,
            (Value::known(line.0.0), Value::known(line.0.1)),
            (Value::known(line.1.0), Value::known(line.1.1)),
            (Value::known(line.2.0), Value::known(line.2.1)),
          )?
        }
      };
      prepared.push(match step {
        MillerStepConstant::Double(_) => MillerStep::Double { line },
        MillerStepConstant::Add(_) => MillerStep::Add { line },
      });
    }

    let miller = miller_loop(&chip, &mut layouter, &point, &PreparedG2Miller::new(prepared))?;
    let actual = final_exponentiation(&chip, &mut layouter, &miller)?;
    let expected = AssignedFp12::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    actual.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}
