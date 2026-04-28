use ff::{Field, PrimeField};
use midnight_circuits::midnight_proofs::circuit::Value;

use super::ForeignField;

mod g2_host;
mod pairing_host;

pub(crate) use g2_host::{
  g1_generator_constant, g2_affine_from_miller_point_constant, g2_affine_from_projective_constant,
  g2_curve_coeff_b_constant, g2_line_evaluation_constant, g2_miller_double_with_line_constant,
  g2_miller_mixed_add_with_line_constant, g2_miller_point_from_affine_constant,
  g2_projective_add_constant, g2_projective_double_constant, g2_projective_from_affine_constant,
  g2_projective_identity_constant,
};
pub(crate) use pairing_host::{
  bn254_final_exponentiation_constant, bn254_final_exponentiation_easy_part_constant,
  bn254_final_exponentiation_hard_part_constant,
};

pub(crate) type Fp2Value = (Value<ForeignField>, Value<ForeignField>);
pub(crate) type Fp2Constant = (ForeignField, ForeignField);
pub(crate) type Fp6Value = (Fp2Value, Fp2Value, Fp2Value);
pub(crate) type Fp6Constant = (Fp2Constant, Fp2Constant, Fp2Constant);
pub(crate) type Fp12Value = (Fp6Value, Fp6Value);
pub(crate) type Fp12Constant = (Fp6Constant, Fp6Constant);
pub(crate) type CompressedCyclotomicFp12Constant =
  (Fp2Constant, Fp2Constant, Fp2Constant, Fp2Constant);
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

pub(crate) fn fp2_frobenius_map_constant(value: Fp2Constant, power: usize) -> Fp2Constant {
  if power % 2 == 0 { value } else { (value.0, -value.1) }
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

pub(crate) fn fp6_inv_constant(value: Fp6Constant) -> Fp6Constant {
  let t0 = fp2_sub_constant(
    fp2_square_constant(value.0),
    fp2_mul_by_fp6_nonresidue_constant(fp2_mul_constant(value.1, value.2)),
  );
  let t1 = fp2_sub_constant(
    fp2_mul_by_fp6_nonresidue_constant(fp2_square_constant(value.2)),
    fp2_mul_constant(value.0, value.1),
  );
  let t2 = fp2_sub_constant(fp2_square_constant(value.1), fp2_mul_constant(value.0, value.2));

  let denom = fp2_add_constant(
    fp2_mul_constant(value.0, t0),
    fp2_mul_by_fp6_nonresidue_constant(fp2_add_constant(
      fp2_mul_constant(value.2, t1),
      fp2_mul_constant(value.1, t2),
    )),
  );
  let denom_inv = fp2_inv_constant(denom);

  (
    fp2_mul_constant(t0, denom_inv),
    fp2_mul_constant(t1, denom_inv),
    fp2_mul_constant(t2, denom_inv),
  )
}

fn fp6_frobenius_coeff_c1(power: usize) -> Fp2Constant {
  match power % 6 {
    0 => (ForeignField::ONE, ForeignField::ZERO),
    1 => (
      ForeignField::from_str_vartime(
        "21575463638280843010398324269430826099269044274347216827212613867836435027261",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c1[1].c0 should parse"),
      ForeignField::from_str_vartime(
        "10307601595873709700152284273816112264069230130616436755625194854815875713954",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c1[1].c1 should parse"),
    ),
    2 => (
      ForeignField::from_str_vartime(
        "21888242871839275220042445260109153167277707414472061641714758635765020556616",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c1[2].c0 should parse"),
      ForeignField::ZERO,
    ),
    3 => (
      ForeignField::from_str_vartime(
        "3772000881919853776433695186713858239009073593817195771773381919316419345261",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c1[3].c0 should parse"),
      ForeignField::from_str_vartime(
        "2236595495967245188281701248203181795121068902605861227855261137820944008926",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c1[3].c1 should parse"),
    ),
    4 => (
      ForeignField::from_str_vartime("2203960485148121921418603742825762020974279258880205651966")
        .expect("hard-coded BN254 Fp6 Frobenius c1[4].c0 should parse"),
      ForeignField::ZERO,
    ),
    5 => (
      ForeignField::from_str_vartime(
        "18429021223477853657660792034369865839114504446431234726392080002137598044644",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c1[5].c0 should parse"),
      ForeignField::from_str_vartime(
        "9344045779998320333812420223237981029506012124075525679208581902008406485703",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c1[5].c1 should parse"),
    ),
    _ => unreachable!(),
  }
}

fn fp6_frobenius_coeff_c2(power: usize) -> Fp2Constant {
  match power % 6 {
    0 => (ForeignField::ONE, ForeignField::ZERO),
    1 => (
      ForeignField::from_str_vartime(
        "2581911344467009335267311115468803099551665605076196740867805258568234346338",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c2[1].c0 should parse"),
      ForeignField::from_str_vartime(
        "19937756971775647987995932169929341994314640652964949448313374472400716661030",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c2[1].c1 should parse"),
    ),
    2 => (
      ForeignField::from_str_vartime("2203960485148121921418603742825762020974279258880205651966")
        .expect("hard-coded BN254 Fp6 Frobenius c2[2].c0 should parse"),
      ForeignField::ZERO,
    ),
    3 => (
      ForeignField::from_str_vartime(
        "5324479202449903542726783395506214481928257762400643279780343368557297135718",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c2[3].c0 should parse"),
      ForeignField::from_str_vartime(
        "16208900380737693084919495127334387981393726419856888799917914180988844123039",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c2[3].c1 should parse"),
    ),
    4 => (
      ForeignField::from_str_vartime(
        "21888242871839275220042445260109153167277707414472061641714758635765020556616",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c2[4].c0 should parse"),
      ForeignField::ZERO,
    ),
    5 => (
      ForeignField::from_str_vartime(
        "13981852324922362344252311234282257507216387789820983642040889267519694726527",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c2[5].c0 should parse"),
      ForeignField::from_str_vartime(
        "7629828391165209371577384193250820201684255241773809077146787135900891633097",
      )
      .expect("hard-coded BN254 Fp6 Frobenius c2[5].c1 should parse"),
    ),
    _ => unreachable!(),
  }
}

pub(crate) fn fp6_scale_by_fp2_constant(value: Fp6Constant, scalar: Fp2Constant) -> Fp6Constant {
  (
    fp2_mul_constant(value.0, scalar),
    fp2_mul_constant(value.1, scalar),
    fp2_mul_constant(value.2, scalar),
  )
}

pub(crate) fn fp6_frobenius_map_constant(value: Fp6Constant, power: usize) -> Fp6Constant {
  let c0 = fp2_frobenius_map_constant(value.0, power);
  let c1 =
    fp2_mul_constant(fp2_frobenius_map_constant(value.1, power), fp6_frobenius_coeff_c1(power));
  let c2 =
    fp2_mul_constant(fp2_frobenius_map_constant(value.2, power), fp6_frobenius_coeff_c2(power));

  (c0, c1, c2)
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

fn fp12_cyclotomic_square_pair_constant(
  left: Fp2Constant,
  right: Fp2Constant,
) -> (Fp2Constant, Fp2Constant) {
  let product = fp2_mul_constant(left, right);
  let left_plus_right = fp2_add_constant(left, right);
  let right_nr = fp2_mul_by_fp6_nonresidue_constant(right);
  let left_plus_right_nr = fp2_add_constant(right_nr, left);
  let product_nr = fp2_mul_by_fp6_nonresidue_constant(product);
  let t0 = fp2_sub_constant(
    fp2_sub_constant(fp2_mul_constant(left_plus_right, left_plus_right_nr), product),
    product_nr,
  );
  let t1 = fp2_add_constant(product, product);

  (t0, t1)
}

fn fp2_three_t_minus_two_z_constant(t: Fp2Constant, z: Fp2Constant) -> Fp2Constant {
  let t_minus_z = fp2_sub_constant(t, z);
  fp2_add_constant(fp2_add_constant(t_minus_z, t_minus_z), t)
}

fn fp2_three_t_plus_two_z_constant(t: Fp2Constant, z: Fp2Constant) -> Fp2Constant {
  let t_plus_z = fp2_add_constant(t, z);
  fp2_add_constant(fp2_add_constant(t_plus_z, t_plus_z), t)
}

/// Squares an Fp12 element under the assumption that it lies in the BN254
/// cyclotomic subgroup reached after the easy part of final exponentiation.
///
/// This implements the Granger-Scott degree-12 cyclotomic squaring formula
/// using the arkworks BN254 tower and must not be used for arbitrary Fp12
/// elements.
pub(crate) fn fp12_cyclotomic_square_constant(value: &Fp12Constant) -> Fp12Constant {
  // arkworks / Granger-Scott coefficient order:
  // z0 = c0.c0, z1 = c1.c1, z2 = c1.c0, z3 = c0.c2, z4 = c0.c1, z5 = c1.c2.
  let (t0, t1) = fp12_cyclotomic_square_pair_constant(value.0.0, value.1.1);
  let (t2, t3) = fp12_cyclotomic_square_pair_constant(value.1.0, value.0.2);
  let (t4, t5) = fp12_cyclotomic_square_pair_constant(value.0.1, value.1.2);

  let z0 = fp2_three_t_minus_two_z_constant(t0, value.0.0);
  let z1 = fp2_three_t_plus_two_z_constant(t1, value.1.1);
  let z2 = fp2_three_t_plus_two_z_constant(fp2_mul_by_fp6_nonresidue_constant(t5), value.1.0);
  let z3 = fp2_three_t_minus_two_z_constant(t4, value.0.2);
  let z4 = fp2_three_t_minus_two_z_constant(t2, value.0.1);
  let z5 = fp2_three_t_plus_two_z_constant(t3, value.1.2);

  ((z0, z4, z3), (z2, z1, z5))
}

pub(crate) fn fp12_cyclotomic_compress_constant(
  value: &Fp12Constant,
) -> CompressedCyclotomicFp12Constant {
  (value.1.0, value.0.2, value.0.1, value.1.2)
}

pub(crate) fn fp12_cyclotomic_square_compressed_constant(
  value: &CompressedCyclotomicFp12Constant,
) -> CompressedCyclotomicFp12Constant {
  let g2 = value.0;
  let g3 = value.1;
  let g4 = value.2;
  let g5 = value.3;

  let b45 = fp2_mul_constant(g4, g5);
  let nr_b45 = fp2_mul_by_fp6_nonresidue_constant(b45);
  let nr_g5 = fp2_mul_by_fp6_nonresidue_constant(g5);
  let a45 = fp2_mul_constant(fp2_add_constant(g4, g5), fp2_add_constant(g4, nr_g5));

  let b23 = fp2_mul_constant(g2, g3);
  let nr_b23 = fp2_mul_by_fp6_nonresidue_constant(b23);
  let nr_g3 = fp2_mul_by_fp6_nonresidue_constant(g3);
  let a23 = fp2_mul_constant(fp2_add_constant(g2, g3), fp2_add_constant(g2, nr_g3));

  let three_nr_b45 = fp2_add_constant(fp2_add_constant(nr_b45, nr_b45), nr_b45);
  let h2 = fp2_add_constant(fp2_add_constant(g2, three_nr_b45), fp2_add_constant(g2, three_nr_b45));

  let ten_plus_u_b45 = fp2_add_constant(nr_b45, b45);
  let a45_minus_ten_b45 = fp2_sub_constant(a45, ten_plus_u_b45);
  let three_a45_minus_ten_b45 =
    fp2_add_constant(fp2_add_constant(a45_minus_ten_b45, a45_minus_ten_b45), a45_minus_ten_b45);
  let h3 = fp2_sub_constant(three_a45_minus_ten_b45, fp2_add_constant(g3, g3));

  let ten_plus_u_b23 = fp2_add_constant(nr_b23, b23);
  let a23_minus_ten_b23 = fp2_sub_constant(a23, ten_plus_u_b23);
  let three_a23_minus_ten_b23 =
    fp2_add_constant(fp2_add_constant(a23_minus_ten_b23, a23_minus_ten_b23), a23_minus_ten_b23);
  let h4 = fp2_sub_constant(three_a23_minus_ten_b23, fp2_add_constant(g4, g4));

  let three_b23 = fp2_add_constant(fp2_add_constant(b23, b23), b23);
  let h5 = fp2_add_constant(fp2_add_constant(g5, three_b23), fp2_add_constant(g5, three_b23));

  (h2, h3, h4, h5)
}

pub(crate) fn fp12_cyclotomic_decompress_constant(
  value: &CompressedCyclotomicFp12Constant,
) -> Fp12Constant {
  let g2 = value.0;
  let g3 = value.1;
  let g4 = value.2;
  let g5 = value.3;

  let g1 = if g2 == (ForeignField::ZERO, ForeignField::ZERO) {
    let numerator = fp2_add_constant(fp2_mul_constant(g4, g5), fp2_mul_constant(g4, g5));
    fp2_mul_constant(numerator, fp2_inv_constant(g3))
  } else {
    let g5_sq_nr = fp2_mul_by_fp6_nonresidue_constant(fp2_square_constant(g5));
    let three_g4_sq = fp2_add_constant(
      fp2_add_constant(fp2_square_constant(g4), fp2_square_constant(g4)),
      fp2_square_constant(g4),
    );
    let numerator =
      fp2_sub_constant(fp2_add_constant(g5_sq_nr, three_g4_sq), fp2_add_constant(g3, g3));
    let four_g2 = fp2_add_constant(fp2_add_constant(g2, g2), fp2_add_constant(g2, g2));
    fp2_mul_constant(numerator, fp2_inv_constant(four_g2))
  };

  let two_g1_sq = fp2_add_constant(fp2_square_constant(g1), fp2_square_constant(g1));
  let three_g3g4 = {
    let g3g4 = fp2_mul_constant(g3, g4);
    fp2_add_constant(fp2_add_constant(g3g4, g3g4), g3g4)
  };
  let inner = if g2 == (ForeignField::ZERO, ForeignField::ZERO) {
    fp2_sub_constant(two_g1_sq, three_g3g4)
  } else {
    fp2_sub_constant(fp2_add_constant(two_g1_sq, fp2_mul_constant(g2, g5)), three_g3g4)
  };
  let g0 = fp2_add_constant(
    fp2_mul_by_fp6_nonresidue_constant(inner),
    (ForeignField::ONE, ForeignField::ZERO),
  );

  ((g0, g4, g3), (g2, g1, g5))
}

pub(crate) fn fp12_conjugate_constant(value: &Fp12Constant) -> Fp12Constant {
  (value.0, fp6_sub_constant(fp6_zero_constant(), value.1))
}

pub(crate) fn fp12_inv_constant(value: &Fp12Constant) -> Fp12Constant {
  let t0 = value.0;
  let t1 = fp6_sub_constant(fp6_zero_constant(), value.1);
  let denom = fp6_sub_constant(
    fp6_square_constant(value.0),
    fp6_mul_by_nonresidue_constant(fp6_square_constant(value.1)),
  );
  let denom_inv = fp6_inv_constant(denom);

  (fp6_mul_constant(t0, denom_inv), fp6_mul_constant(t1, denom_inv))
}

fn fp12_frobenius_coeff_c1(power: usize) -> Fp2Constant {
  match power % 12 {
    0 => (ForeignField::ONE, ForeignField::ZERO),
    1 => (
      ForeignField::from_str_vartime(
        "8376118865763821496583973867626364092589906065868298776909617916018768340080",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[1].c0 should parse"),
      ForeignField::from_str_vartime(
        "16469823323077808223889137241176536799009286646108169935659301613961712198316",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[1].c1 should parse"),
    ),
    2 => (
      ForeignField::from_str_vartime(
        "21888242871839275220042445260109153167277707414472061641714758635765020556617",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[2].c0 should parse"),
      ForeignField::ZERO,
    ),
    3 => (
      ForeignField::from_str_vartime(
        "11697423496358154304825782922584725312912383441159505038794027105778954184319",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[3].c0 should parse"),
      ForeignField::from_str_vartime(
        "303847389135065887422783454877609941456349188919719272345083954437860409601",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[3].c1 should parse"),
    ),
    4 => (
      ForeignField::from_str_vartime(
        "21888242871839275220042445260109153167277707414472061641714758635765020556616",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[4].c0 should parse"),
      ForeignField::ZERO,
    ),
    5 => (
      ForeignField::from_str_vartime(
        "3321304630594332808241809054958361220322477375291206261884409189760185844239",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[5].c0 should parse"),
      ForeignField::from_str_vartime(
        "5722266937896532885780051958958348231143373700109372999374820235121374419868",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[5].c1 should parse"),
    ),
    6 => (-ForeignField::ONE, ForeignField::ZERO),
    7 => (
      ForeignField::from_str_vartime(
        "13512124006075453725662431877630910996106405091429524885779419978626457868503",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[7].c0 should parse"),
      ForeignField::from_str_vartime(
        "5418419548761466998357268504080738289687024511189653727029736280683514010267",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[7].c1 should parse"),
    ),
    8 => (
      ForeignField::from_str_vartime("2203960485148121921418603742825762020974279258880205651966")
        .expect("hard-coded BN254 Fp12 Frobenius c1[8].c0 should parse"),
      ForeignField::ZERO,
    ),
    9 => (
      ForeignField::from_str_vartime(
        "10190819375481120917420622822672549775783927716138318623895010788866272024264",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[9].c0 should parse"),
      ForeignField::from_str_vartime(
        "21584395482704209334823622290379665147239961968378104390343953940207365798982",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[9].c1 should parse"),
    ),
    10 => (
      ForeignField::from_str_vartime("2203960485148121921418603742825762020974279258880205651967")
        .expect("hard-coded BN254 Fp12 Frobenius c1[10].c0 should parse"),
      ForeignField::ZERO,
    ),
    11 => (
      ForeignField::from_str_vartime(
        "18566938241244942414004596690298913868373833782006617400804628704885040364344",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[11].c0 should parse"),
      ForeignField::from_str_vartime(
        "16165975933942742336466353786298926857552937457188450663314217659523851788715",
      )
      .expect("hard-coded BN254 Fp12 Frobenius c1[11].c1 should parse"),
    ),
    _ => unreachable!(),
  }
}

pub(crate) fn fp12_frobenius_map_constant(value: &Fp12Constant, power: usize) -> Fp12Constant {
  let c0 = fp6_frobenius_map_constant(value.0, power);
  let c1 = fp6_scale_by_fp2_constant(
    fp6_frobenius_map_constant(value.1, power),
    fp12_frobenius_coeff_c1(power),
  );

  (c0, c1)
}

pub(crate) fn fp6_zero_constant() -> Fp6Constant {
  (
    (ForeignField::ZERO, ForeignField::ZERO),
    (ForeignField::ZERO, ForeignField::ZERO),
    (ForeignField::ZERO, ForeignField::ZERO),
  )
}

pub(crate) fn fp12_nonresidue_constant() -> Fp6Constant {
  (
    (ForeignField::ZERO, ForeignField::ZERO),
    (ForeignField::ONE, ForeignField::ZERO),
    (ForeignField::ZERO, ForeignField::ZERO),
  )
}
