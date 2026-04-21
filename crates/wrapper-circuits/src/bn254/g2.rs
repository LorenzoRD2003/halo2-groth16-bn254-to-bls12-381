use ff::{Field, PrimeField};
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{
  AssignedFp, AssignedFp2, AssignedFp6, AssignedFp12, Bn254FieldChip, Bn254FieldConfig,
  ForeignField, NativeField,
};

type Fp2Value = (Value<ForeignField>, Value<ForeignField>);
type Fp2Constant = (ForeignField, ForeignField);
#[cfg(test)]
type Fp6Value = (Fp2Value, Fp2Value, Fp2Value);
#[cfg(test)]
type Fp6Constant = (Fp2Constant, Fp2Constant, Fp2Constant);
#[cfg(test)]
type Fp12Value = (Fp6Value, Fp6Value);
#[cfg(test)]
type Fp12Constant = (Fp6Constant, Fp6Constant);
type G2AffineValue = (Fp2Value, Fp2Value);
type G2AffineConstant = (Fp2Constant, Fp2Constant);
type G2ProjectiveConstant = (Fp2Constant, Fp2Constant, Fp2Constant);
type G2MillerPointConstant = (Fp2Constant, Fp2Constant, Fp2Constant);
type G2LineCoeffsConstant = (Fp2Constant, Fp2Constant, Fp2Constant);
#[cfg(test)]
type G2LineCoeffsValue = (Fp2Value, Fp2Value, Fp2Value);

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
  let (x_coord, y_coord, z_coord) = g2_projective_from_affine_constant(point);
  let x_sq = fp2_square_constant(x_coord);
  let y_sq = fp2_square_constant(y_coord);
  let y_fourth = fp2_square_constant(y_sq);
  let slope_intermediate = {
    let x_plus_y_sq = fp2_add_constant(x_coord, y_sq);
    let x_plus_y_sq_sq = fp2_square_constant(x_plus_y_sq);
    let slope_intermediate = fp2_sub_constant(fp2_sub_constant(x_plus_y_sq_sq, x_sq), y_fourth);
    fp2_add_constant(slope_intermediate, slope_intermediate)
  };
  let slope = fp2_add_constant(fp2_add_constant(x_sq, x_sq), x_sq);
  let slope_sq = fp2_square_constant(slope);
  let x3 = fp2_sub_constant(slope_sq, fp2_add_constant(slope_intermediate, slope_intermediate));
  let y3 = {
    let slope_times_delta = fp2_mul_constant(slope, fp2_sub_constant(slope_intermediate, x3));
    let two_y_fourth = fp2_add_constant(y_fourth, y_fourth);
    let four_y_fourth = fp2_add_constant(two_y_fourth, two_y_fourth);
    let eight_y_fourth = fp2_add_constant(four_y_fourth, four_y_fourth);
    fp2_sub_constant(slope_times_delta, eight_y_fourth)
  };
  let yz = fp2_mul_constant(y_coord, z_coord);
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
  let x_diff = fp2_sub_constant(u2, u1);
  let x_diff_twice_sq = fp2_square_constant(fp2_add_constant(x_diff, x_diff));
  let x_diff_cubed_scaled = fp2_mul_constant(x_diff, x_diff_twice_sq);
  let y_diff_twice = fp2_add_constant(fp2_sub_constant(s2, s1), fp2_sub_constant(s2, s1));
  let u1_times_scale = fp2_mul_constant(u1, x_diff_twice_sq);
  let x3 = fp2_sub_constant(
    fp2_sub_constant(fp2_square_constant(y_diff_twice), x_diff_cubed_scaled),
    fp2_add_constant(u1_times_scale, u1_times_scale),
  );
  let y3 = {
    let y_slope_times_delta = fp2_mul_constant(y_diff_twice, fp2_sub_constant(u1_times_scale, x3));
    let two_s1_scale = fp2_add_constant(
      fp2_mul_constant(s1, x_diff_cubed_scaled),
      fp2_mul_constant(s1, x_diff_cubed_scaled),
    );
    fp2_sub_constant(y_slope_times_delta, two_s1_scale)
  };
  let z3 = {
    let z1_plus_z2 = fp2_add_constant(z1, z2);
    let z1_plus_z2_sq = fp2_square_constant(z1_plus_z2);
    let z3_pre = fp2_sub_constant(fp2_sub_constant(z1_plus_z2_sq, z1z1), z2z2);
    fp2_mul_constant(z3_pre, x_diff)
  };

  (x3, y3, z3)
}

fn g2_miller_point_from_affine_constant(point: G2AffineConstant) -> G2MillerPointConstant {
  (point.0, point.1, (ForeignField::ONE, ForeignField::ZERO))
}

fn fp2_inv_constant(value: Fp2Constant) -> Fp2Constant {
  let norm = value.0.square() + value.1.square();
  let norm_inv = norm.invert().expect("nonzero Fp2 norm should be invertible");
  (value.0 * norm_inv, -value.1 * norm_inv)
}

fn g2_affine_from_miller_point_constant(point: G2MillerPointConstant) -> G2AffineConstant {
  let z_inv = fp2_inv_constant(point.2);
  (fp2_mul_constant(point.0, z_inv), fp2_mul_constant(point.1, z_inv))
}

fn g2_miller_double_with_line_constant(
  point: G2MillerPointConstant,
) -> (G2MillerPointConstant, G2LineCoeffsConstant) {
  let mut current = point;

  // Arkworks BN G2 prepared-step formulas over homogeneous projective coordinates.
  let mut xy_half = fp2_mul_constant(current.0, current.1);
  let two_inv =
    ForeignField::from(2_u64).invert().expect("hard-coded base-field two should be invertible");
  xy_half = (xy_half.0 * two_inv, xy_half.1 * two_inv);
  let y_square = fp2_square_constant(current.1);
  let z_square = fp2_square_constant(current.2);
  let twist_times_three_z_square = fp2_mul_constant(
    g2_curve_coeff_b(),
    fp2_add_constant(fp2_add_constant(z_square, z_square), z_square),
  );
  let triple_twist_term = fp2_add_constant(
    fp2_add_constant(twist_times_three_z_square, twist_times_three_z_square),
    twist_times_three_z_square,
  );
  let mut average_y_square_and_twist = fp2_add_constant(y_square, triple_twist_term);
  average_y_square_and_twist =
    (average_y_square_and_twist.0 * two_inv, average_y_square_and_twist.1 * two_inv);
  let y_plus_z_cross = fp2_sub_constant(
    fp2_square_constant(fp2_add_constant(current.1, current.2)),
    fp2_add_constant(y_square, z_square),
  );
  let vertical_term = fp2_sub_constant(twist_times_three_z_square, y_square);
  let x_square = fp2_square_constant(current.0);
  let twist_term_square = fp2_square_constant(twist_times_three_z_square);

  current.0 = fp2_mul_constant(xy_half, fp2_sub_constant(y_square, triple_twist_term));
  current.1 = fp2_sub_constant(
    fp2_square_constant(average_y_square_and_twist),
    fp2_add_constant(fp2_add_constant(twist_term_square, twist_term_square), twist_term_square),
  );
  current.2 = fp2_mul_constant(y_square, y_plus_z_cross);

  let line = (
    fp2_neg_constant(y_plus_z_cross),
    fp2_add_constant(fp2_add_constant(x_square, x_square), x_square),
    vertical_term,
  );

  (current, line)
}

fn g2_miller_mixed_add_with_line_constant(
  point: G2MillerPointConstant,
  addend: G2AffineConstant,
) -> (G2MillerPointConstant, G2LineCoeffsConstant) {
  let mut current = point;

  // Arkworks BN G2 prepared-step formulas over homogeneous projective coordinates.
  let theta = fp2_sub_constant(current.1, fp2_mul_constant(addend.1, current.2));
  let lambda = fp2_sub_constant(current.0, fp2_mul_constant(addend.0, current.2));
  let theta_square = fp2_square_constant(theta);
  let lambda_square = fp2_square_constant(lambda);
  let lambda_cubed = fp2_mul_constant(lambda, lambda_square);
  let z_times_theta_square = fp2_mul_constant(current.2, theta_square);
  let x_times_lambda_square = fp2_mul_constant(current.0, lambda_square);
  let next_x_intermediate = fp2_sub_constant(
    fp2_add_constant(lambda_cubed, z_times_theta_square),
    fp2_add_constant(x_times_lambda_square, x_times_lambda_square),
  );

  current.0 = fp2_mul_constant(lambda, next_x_intermediate);
  current.1 = fp2_sub_constant(
    fp2_mul_constant(theta, fp2_sub_constant(x_times_lambda_square, next_x_intermediate)),
    fp2_mul_constant(lambda_cubed, current.1),
  );
  current.2 = fp2_mul_constant(current.2, lambda_cubed);

  let constant_term =
    fp2_sub_constant(fp2_mul_constant(theta, addend.0), fp2_mul_constant(lambda, addend.1));
  let line = (lambda, fp2_neg_constant(theta), constant_term);

  (current, line)
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
    let x_sq = self.x.square(chip, layouter)?;
    let y_sq = self.y.square(chip, layouter)?;
    let y_fourth = y_sq.square(chip, layouter)?;
    let slope_intermediate = {
      let x_plus_y_sq = self.x.add(chip, layouter, &y_sq)?;
      let x_plus_y_sq_sq = x_plus_y_sq.square(chip, layouter)?;
      let slope_intermediate =
        x_plus_y_sq_sq.sub(chip, layouter, &x_sq)?.sub(chip, layouter, &y_fourth)?;
      slope_intermediate.add(chip, layouter, &slope_intermediate)?
    };
    let slope = {
      let two_x_sq = x_sq.add(chip, layouter, &x_sq)?;
      two_x_sq.add(chip, layouter, &x_sq)?
    };
    let slope_sq = slope.square(chip, layouter)?;
    let x3 = {
      let two_slope_intermediate = slope_intermediate.add(chip, layouter, &slope_intermediate)?;
      slope_sq.sub(chip, layouter, &two_slope_intermediate)?
    };
    let y3 = {
      let delta = slope_intermediate.sub(chip, layouter, &x3)?;
      let slope_times_delta = slope.mul(chip, layouter, &delta)?;
      let two_y_fourth = y_fourth.add(chip, layouter, &y_fourth)?;
      let four_y_fourth = two_y_fourth.add(chip, layouter, &two_y_fourth)?;
      let eight_y_fourth = four_y_fourth.add(chip, layouter, &four_y_fourth)?;
      slope_times_delta.sub(chip, layouter, &eight_y_fourth)?
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
    let x3 = {
      let y_diff_twice_sq = y_diff_twice.square(chip, layouter)?;
      let two_u1_times_scale = u1_times_scale.add(chip, layouter, &u1_times_scale)?;
      y_diff_twice_sq.sub(chip, layouter, &x_diff_cubed_scaled)?.sub(
        chip,
        layouter,
        &two_u1_times_scale,
      )?
    };
    let y3 = {
      let delta = u1_times_scale.sub(chip, layouter, &x3)?;
      let y_slope_times_delta = y_diff_twice.mul(chip, layouter, &delta)?;
      let s1_scaled = s1.mul(chip, layouter, &x_diff_cubed_scaled)?;
      let two_s1_scaled = s1_scaled.add(chip, layouter, &s1_scaled)?;
      y_slope_times_delta.sub(chip, layouter, &two_s1_scaled)?
    };
    let z3 = {
      let z1_plus_z2 = self.z.add(chip, layouter, &rhs.z)?;
      let z1_plus_z2_sq = z1_plus_z2.square(chip, layouter)?;
      let z3_pre = z1_plus_z2_sq.sub(chip, layouter, &z1z1)?.sub(chip, layouter, &z2z2)?;
      z3_pre.mul(chip, layouter, &x_diff)?
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

  /// Evaluates the sparse line coefficients at a G1 affine point and embeds the result into Fp12.
  ///
  /// The resulting sparse Fp12 value is ready for later Miller-loop accumulator multiplication.
  ///
  /// # Errors
  ///
  /// Returns an error if the required Fp or Fp2 operations fail.
  pub fn evaluate_at_g1(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    g1_x: &AssignedFp,
    g1_y: &AssignedFp,
  ) -> Result<AssignedFp12, Error> {
    let slot_c0 = self.ell_0.scale_by_fp(chip, layouter, g1_y)?;
    let slot_c3 = self.ell_w.scale_by_fp(chip, layouter, g1_x)?;
    let zero_fp2 = AssignedFp2::zero(chip, layouter)?;

    Ok(AssignedFp12::new(
      AssignedFp6::new(slot_c0, zero_fp2.clone(), zero_fp2.clone()),
      AssignedFp6::new(slot_c3, self.ell_vw.clone(), zero_fp2),
    ))
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
    let two_inv = chip.assign(
      layouter,
      Value::known(
        ForeignField::from(2_u64)
          .invert()
          .expect("hard-coded BN254 base-field two should be invertible"),
      ),
    )?;

    let xy = self.x.mul(chip, layouter, &self.y)?;
    let xy_half = xy.scale_by_fp(chip, layouter, &two_inv)?;
    let y_square = self.y.square(chip, layouter)?;
    let z_square = self.z.square(chip, layouter)?;
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
    let y_plus_z = self.y.add(chip, layouter, &self.z)?;
    let y_plus_z_sq = y_plus_z.square(chip, layouter)?;
    let y_plus_z_sum = y_square.add(chip, layouter, &z_square)?;
    let y_minus_twist_term = y_square.sub(chip, layouter, &triple_twist_term)?;
    let vertical_term = twist_times_three_z_square.sub(chip, layouter, &y_square)?;
    let x_square = self.x.square(chip, layouter)?;
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

    Ok((Self::new(next_x, next_y, next_z), line))
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
    let rhs_y_times_z = rhs.y.mul(chip, layouter, &self.z)?;
    let rhs_x_times_z = rhs.x.mul(chip, layouter, &self.z)?;
    let theta = self.y.sub(chip, layouter, &rhs_y_times_z)?;
    let lambda = self.x.sub(chip, layouter, &rhs_x_times_z)?;
    let theta_square = theta.square(chip, layouter)?;
    let lambda_square = lambda.square(chip, layouter)?;
    let lambda_cubed = lambda.mul(chip, layouter, &lambda_square)?;
    let z_times_theta_square = self.z.mul(chip, layouter, &theta_square)?;
    let x_times_lambda_square = self.x.mul(chip, layouter, &lambda_square)?;
    let double_x_lambda_square =
      x_times_lambda_square.add(chip, layouter, &x_times_lambda_square)?;
    let next_x_intermediate = lambda_cubed.add(chip, layouter, &z_times_theta_square)?.sub(
      chip,
      layouter,
      &double_x_lambda_square,
    )?;

    let next_x = lambda.mul(chip, layouter, &next_x_intermediate)?;
    let x_delta = x_times_lambda_square.sub(chip, layouter, &next_x_intermediate)?;
    let theta_times_delta = theta.mul(chip, layouter, &x_delta)?;
    let lambda_cubed_times_y = lambda_cubed.mul(chip, layouter, &self.y)?;
    let next_y = theta_times_delta.sub(chip, layouter, &lambda_cubed_times_y)?;
    let next_z = self.z.mul(chip, layouter, &lambda_cubed)?;
    let theta_times_rhs_x = theta.mul(chip, layouter, &rhs.x)?;
    let lambda_times_rhs_y = lambda.mul(chip, layouter, &rhs.y)?;
    let line_constant_term = theta_times_rhs_x.sub(chip, layouter, &lambda_times_rhs_y)?;

    let line =
      AssignedG2LineCoeffs::new(lambda.clone(), theta.neg(chip, layouter)?, line_constant_term);

    Ok((Self::new(next_x, next_y, next_z), line))
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

/// Small circuit that evaluates sparse Miller-path line coefficients into an Fp12 value.
#[cfg(test)]
#[derive(Clone, Debug)]
pub struct G2LineEvaluationCircuit {
  line: G2LineCoeffsValue,
  g1_x: Value<ForeignField>,
  g1_y: Value<ForeignField>,
  expected: Fp12Value,
}

#[cfg(test)]
impl G2LineEvaluationCircuit {
  /// Builds a new sparse line-evaluation circuit with a known Fp12 output.
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
impl Circuit<NativeField> for G2LineEvaluationCircuit {
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
    let evaluated = line.evaluate_at_g1(&chip, &mut layouter, &g1_x, &g1_y)?;
    let expected = AssignedFp12::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    evaluated.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}
