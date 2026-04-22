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

pub(crate) fn fp12_pow_constant_exp(value: &Fp12Constant, exp: &[u64]) -> Fp12Constant {
  let mut result = fp12_one_constant();
  let mut seen_one = false;

  for limb in exp.iter().rev() {
    for bit in (0..64).rev() {
      if seen_one {
        result = fp12_square_constant(&result);
      }
      if ((*limb >> bit) & 1) == 1 {
        seen_one = true;
        result = fp12_mul_constant(&result, value);
      }
    }
  }

  result
}

pub(crate) fn fp12_exp_by_neg_x_constant(value: &Fp12Constant) -> Fp12Constant {
  let exp = fp12_pow_constant_exp(value, &[4965661367192848881]);
  fp12_conjugate_constant(&exp)
}

pub(crate) fn bn254_final_exponentiation_constant(value: &Fp12Constant) -> Fp12Constant {
  let f1 = fp12_conjugate_constant(value);
  let f2 = fp12_inv_constant(value);
  let mut r = fp12_mul_constant(&f1, &f2);
  let r_clone = r;
  r = fp12_frobenius_map_constant(&r, 2);
  r = fp12_mul_constant(&r, &r_clone);

  let y0 = fp12_exp_by_neg_x_constant(&r);
  let y1 = fp12_square_constant(&y0);
  let y2 = fp12_square_constant(&y1);
  let mut y3 = fp12_mul_constant(&y2, &y1);
  let y4 = fp12_exp_by_neg_x_constant(&y3);
  let y5 = fp12_square_constant(&y4);
  let mut y6 = fp12_exp_by_neg_x_constant(&y5);
  y3 = fp12_conjugate_constant(&y3);
  y6 = fp12_conjugate_constant(&y6);
  let y7 = fp12_mul_constant(&y6, &y4);
  let mut y8 = fp12_mul_constant(&y7, &y3);
  let y9 = fp12_mul_constant(&y8, &y1);
  let y10 = fp12_mul_constant(&y8, &y4);
  let y11 = fp12_mul_constant(&y10, &r);
  let mut y12 = fp12_frobenius_map_constant(&y9, 1);
  y12 = fp12_mul_constant(&y12, &y11);
  y8 = fp12_frobenius_map_constant(&y8, 2);
  let y14 = fp12_mul_constant(&y8, &y12);
  let r_inv = fp12_conjugate_constant(&r);
  let mut y15 = fp12_mul_constant(&r_inv, &y9);
  y15 = fp12_frobenius_map_constant(&y15, 3);
  fp12_mul_constant(&y15, &y14)
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
