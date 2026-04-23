use super::*;

const BN254_X_ABS: u64 = 4_965_661_367_192_848_881;

#[cfg(test)]
fn fp12_pow_constant_exp(value: &Fp12Constant, exp: &[u64]) -> Fp12Constant {
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

fn fp12_square_6_times(value: &Fp12Constant) -> Fp12Constant {
  let value = fp12_square_constant(value);
  let value = fp12_square_constant(&value);
  let value = fp12_square_constant(&value);
  let value = fp12_square_constant(&value);
  let value = fp12_square_constant(&value);
  fp12_square_constant(&value)
}

fn fp12_square_7_times(value: &Fp12Constant) -> Fp12Constant {
  let value = fp12_square_6_times(value);
  fp12_square_constant(&value)
}

fn fp12_square_8_times(value: &Fp12Constant) -> Fp12Constant {
  let value = fp12_square_7_times(value);
  fp12_square_constant(&value)
}

fn fp12_square_10_times(value: &Fp12Constant) -> Fp12Constant {
  let value = fp12_square_8_times(value);
  let value = fp12_square_constant(&value);
  fp12_square_constant(&value)
}

pub(crate) fn fp12_exp_by_neg_x_constant(value: &Fp12Constant) -> Fp12Constant {
  // Compute value^x for the BN254 parameter
  // x = 0x44e992b44a6909f1 = 4965661367192848881.
  //
  // This mirrors the circuit-side fixed chain exactly:
  // x = ((((((((17 << 7) + 29) << 7) + 25) << 8) + 43) << 6) + 17) << 8
  //    + 41) << 6 + 41) << 10 + 39) << 6 + 49.
  //
  // Relative to generic square-and-multiply for this exponent, the chain uses
  // 63 squares + 16 muls instead of 62 squares + 27 muls before the final
  // conjugation.
  debug_assert_eq!(BN254_X_ABS, 0x44e9_92b4_4a69_09f1);
  let x2 = fp12_square_constant(value);
  let x4 = fp12_square_constant(&x2);
  let x8 = fp12_square_constant(&x4);
  let x16 = fp12_square_constant(&x8);
  let x32 = fp12_square_constant(&x16);

  let x10 = fp12_mul_constant(&x8, &x2);
  let x17 = fp12_mul_constant(&x16, value);
  let x25 = fp12_mul_constant(&x17, &x8);
  let x29 = fp12_mul_constant(&x25, &x4);
  let x39 = fp12_mul_constant(&x29, &x10);
  let x41 = fp12_mul_constant(&x25, &x16);
  let x43 = fp12_mul_constant(&x41, &x2);
  let x49 = fp12_mul_constant(&x32, &x17);

  let mut exp = x17;
  exp = fp12_square_7_times(&exp);
  exp = fp12_mul_constant(&exp, &x29);
  exp = fp12_square_7_times(&exp);
  exp = fp12_mul_constant(&exp, &x25);
  exp = fp12_square_8_times(&exp);
  exp = fp12_mul_constant(&exp, &x43);
  exp = fp12_square_6_times(&exp);
  exp = fp12_mul_constant(&exp, &x17);
  exp = fp12_square_8_times(&exp);
  exp = fp12_mul_constant(&exp, &x41);
  exp = fp12_square_6_times(&exp);
  exp = fp12_mul_constant(&exp, &x41);
  exp = fp12_square_10_times(&exp);
  exp = fp12_mul_constant(&exp, &x39);
  exp = fp12_square_6_times(&exp);
  exp = fp12_mul_constant(&exp, &x49);
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn exp_by_neg_x_constant_matches_generic_square_and_multiply() {
    let sample = (
      (
        (ForeignField::from(1_u64), ForeignField::from(2_u64)),
        (ForeignField::from(3_u64), ForeignField::from(4_u64)),
        (ForeignField::from(5_u64), ForeignField::from(6_u64)),
      ),
      (
        (ForeignField::from(7_u64), ForeignField::from(8_u64)),
        (ForeignField::from(9_u64), ForeignField::from(10_u64)),
        (ForeignField::from(11_u64), ForeignField::from(12_u64)),
      ),
    );
    let expected = fp12_conjugate_constant(&fp12_pow_constant_exp(&sample, &[BN254_X_ABS]));
    let actual = fp12_exp_by_neg_x_constant(&sample);

    assert_eq!(actual, expected);
  }
}
