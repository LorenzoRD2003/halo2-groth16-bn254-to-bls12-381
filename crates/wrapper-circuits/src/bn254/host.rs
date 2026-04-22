use ff::{Field, PrimeField};
use midnight_circuits::midnight_proofs::circuit::Value;

use super::ForeignField;

pub(crate) type Fp2Value = (Value<ForeignField>, Value<ForeignField>);
pub(crate) type Fp2Constant = (ForeignField, ForeignField);
pub(crate) type Fp6Value = (Fp2Value, Fp2Value, Fp2Value);
pub(crate) type Fp6Constant = (Fp2Constant, Fp2Constant, Fp2Constant);
pub(crate) type Fp12Value = (Fp6Value, Fp6Value);
pub(crate) type Fp12Constant = (Fp6Constant, Fp6Constant);
pub(crate) type G2AffineConstant = (Fp2Constant, Fp2Constant);
pub(crate) type G2ProjectiveConstant = (Fp2Constant, Fp2Constant, Fp2Constant);
pub(crate) type G2MillerPointConstant = (Fp2Constant, Fp2Constant, Fp2Constant);
pub(crate) type G2LineCoeffsConstant = (Fp2Constant, Fp2Constant, Fp2Constant);

pub(crate) fn fp12_one_constant() -> Fp12Constant {
  (
    (
      (ForeignField::ONE, ForeignField::ZERO),
      (ForeignField::ZERO, ForeignField::ZERO),
      (ForeignField::ZERO, ForeignField::ZERO),
    ),
    (
      (ForeignField::ZERO, ForeignField::ZERO),
      (ForeignField::ZERO, ForeignField::ZERO),
      (ForeignField::ZERO, ForeignField::ZERO),
    ),
  )
}

pub(crate) fn fp2_add_constant(left: Fp2Constant, right: Fp2Constant) -> Fp2Constant {
  (left.0 + right.0, left.1 + right.1)
}

pub(crate) fn fp2_sub_constant(left: Fp2Constant, right: Fp2Constant) -> Fp2Constant {
  (left.0 - right.0, left.1 - right.1)
}

pub(crate) fn fp2_neg_constant(value: Fp2Constant) -> Fp2Constant {
  (-value.0, -value.1)
}

pub(crate) fn fp2_mul_constant(left: Fp2Constant, right: Fp2Constant) -> Fp2Constant {
  let ac = left.0 * right.0;
  let bd = left.1 * right.1;
  let ad = left.0 * right.1;
  let bc = left.1 * right.0;

  (ac - bd, ad + bc)
}

pub(crate) fn fp2_square_constant(value: Fp2Constant) -> Fp2Constant {
  let a_sq = value.0.square();
  let b_sq = value.1.square();
  let ab = value.0 * value.1;

  (a_sq - b_sq, ab + ab)
}

pub(crate) fn fp2_inv_constant(value: Fp2Constant) -> Fp2Constant {
  let norm = value.0.square() + value.1.square();
  let norm_inv = norm.invert().expect("nonzero Fp2 norm should be invertible");
  (value.0 * norm_inv, -value.1 * norm_inv)
}

pub(crate) fn fp6_nonresidue_constant() -> Fp2Constant {
  (
    ForeignField::from_str_vartime("9").expect("hard-coded BN254 Fp6 nonresidue c0 should parse"),
    ForeignField::ONE,
  )
}

pub(crate) fn fp2_mul_by_fp6_nonresidue_constant(value: Fp2Constant) -> Fp2Constant {
  let nine_c0 = value.0 * ForeignField::from(9_u64);
  let nine_c1 = value.1 * ForeignField::from(9_u64);
  (nine_c0 - value.1, nine_c1 + value.0)
}

pub(crate) fn fp6_add_constant(left: Fp6Constant, right: Fp6Constant) -> Fp6Constant {
  (
    fp2_add_constant(left.0, right.0),
    fp2_add_constant(left.1, right.1),
    fp2_add_constant(left.2, right.2),
  )
}

pub(crate) fn fp6_sub_constant(left: Fp6Constant, right: Fp6Constant) -> Fp6Constant {
  (
    fp2_sub_constant(left.0, right.0),
    fp2_sub_constant(left.1, right.1),
    fp2_sub_constant(left.2, right.2),
  )
}

pub(crate) fn fp6_mul_by_nonresidue_constant(value: Fp6Constant) -> Fp6Constant {
  (fp2_mul_by_fp6_nonresidue_constant(value.2), value.0, value.1)
}

pub(crate) fn fp6_mul_constant(left: Fp6Constant, right: Fp6Constant) -> Fp6Constant {
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

pub(crate) fn fp6_square_constant(value: Fp6Constant) -> Fp6Constant {
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

pub(crate) fn fp12_add_constant(left: &Fp12Constant, right: &Fp12Constant) -> Fp12Constant {
  (fp6_add_constant(left.0, right.0), fp6_add_constant(left.1, right.1))
}

pub(crate) fn fp12_mul_constant(left: &Fp12Constant, right: &Fp12Constant) -> Fp12Constant {
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

pub(crate) fn fp12_square_constant(value: &Fp12Constant) -> Fp12Constant {
  let a_sq = fp6_square_constant(value.0);
  let b_sq = fp6_square_constant(value.1);
  let ab = fp6_mul_constant(value.0, value.1);

  (fp6_add_constant(a_sq, fp6_mul_by_nonresidue_constant(b_sq)), fp6_add_constant(ab, ab))
}

pub(crate) fn fp12_nonresidue_constant() -> Fp6Constant {
  (
    (ForeignField::ZERO, ForeignField::ZERO),
    (ForeignField::ONE, ForeignField::ZERO),
    (ForeignField::ZERO, ForeignField::ZERO),
  )
}

pub(crate) fn g1_generator_constant() -> (ForeignField, ForeignField) {
  (ForeignField::ONE, ForeignField::from(2_u64))
}

pub(crate) fn g2_line_evaluation_constant(
  line: G2LineCoeffsConstant,
  g1: (ForeignField, ForeignField),
) -> Fp12Constant {
  (
    (
      (line.0.0 * g1.1, line.0.1 * g1.1),
      (ForeignField::ZERO, ForeignField::ZERO),
      (ForeignField::ZERO, ForeignField::ZERO),
    ),
    ((line.1.0 * g1.0, line.1.1 * g1.0), line.2, (ForeignField::ZERO, ForeignField::ZERO)),
  )
}

pub(crate) fn g2_curve_coeff_b_constant() -> Fp2Constant {
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

pub(crate) fn g2_projective_identity_constant() -> G2ProjectiveConstant {
  (
    (ForeignField::ONE, ForeignField::ZERO),
    (ForeignField::ONE, ForeignField::ZERO),
    (ForeignField::ZERO, ForeignField::ZERO),
  )
}

pub(crate) fn g2_projective_from_affine_constant(point: G2AffineConstant) -> G2ProjectiveConstant {
  (point.0, point.1, (ForeignField::ONE, ForeignField::ZERO))
}

pub(crate) fn g2_projective_double_constant(point: G2AffineConstant) -> G2ProjectiveConstant {
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

pub(crate) fn g2_projective_add_constant(
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

pub(crate) fn g2_miller_point_from_affine_constant(
  point: G2AffineConstant,
) -> G2MillerPointConstant {
  (point.0, point.1, (ForeignField::ONE, ForeignField::ZERO))
}

pub(crate) fn g2_affine_from_miller_point_constant(
  point: G2MillerPointConstant,
) -> G2AffineConstant {
  let z_inv = fp2_inv_constant(point.2);
  (fp2_mul_constant(point.0, z_inv), fp2_mul_constant(point.1, z_inv))
}

pub(crate) fn g2_miller_double_with_line_constant(
  point: G2MillerPointConstant,
) -> (G2MillerPointConstant, G2LineCoeffsConstant) {
  let mut current = point;

  let mut xy_half = fp2_mul_constant(current.0, current.1);
  let two_inv =
    ForeignField::from(2_u64).invert().expect("hard-coded base-field two should be invertible");
  xy_half = (xy_half.0 * two_inv, xy_half.1 * two_inv);
  let y_square = fp2_square_constant(current.1);
  let z_square = fp2_square_constant(current.2);
  let twist_times_three_z_square = fp2_mul_constant(
    g2_curve_coeff_b_constant(),
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

pub(crate) fn g2_miller_mixed_add_with_line_constant(
  point: G2MillerPointConstant,
  addend: G2AffineConstant,
) -> (G2MillerPointConstant, G2LineCoeffsConstant) {
  let mut current = point;

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
