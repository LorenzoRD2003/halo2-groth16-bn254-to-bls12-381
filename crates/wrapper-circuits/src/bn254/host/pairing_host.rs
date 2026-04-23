use super::*;

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

pub(crate) fn bn254_final_exponentiation_easy_part_constant(value: &Fp12Constant) -> Fp12Constant {
  let f1 = fp12_conjugate_constant(value);
  let f2 = fp12_inv_constant(value);
  let mut r = fp12_mul_constant(&f1, &f2);
  let r_clone = r;
  r = fp12_frobenius_map_constant(&r, 2);
  fp12_mul_constant(&r, &r_clone)
}

pub(crate) fn bn254_final_exponentiation_hard_part_constant(value: &Fp12Constant) -> Fp12Constant {
  let r = *value;

  let y0 = fp12_exp_by_neg_x_constant(value);
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

pub(crate) fn bn254_final_exponentiation_constant(value: &Fp12Constant) -> Fp12Constant {
  let easy = bn254_final_exponentiation_easy_part_constant(value);
  bn254_final_exponentiation_hard_part_constant(&easy)
}
