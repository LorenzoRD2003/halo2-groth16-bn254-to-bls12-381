use super::*;
use crate::bn254::{
  BN254_EXP_BY_X_CHAIN_START, BN254_EXP_BY_X_CHAIN_STEPS, BN254_X_ABS, Bn254ExpByXWindow,
};

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

fn fp12_cyclotomic_square_6_times(value: &Fp12Constant) -> Fp12Constant {
  let value = fp12_cyclotomic_square_constant(value);
  let value = fp12_cyclotomic_square_constant(&value);
  let value = fp12_cyclotomic_square_constant(&value);
  let value = fp12_cyclotomic_square_constant(&value);
  let value = fp12_cyclotomic_square_constant(&value);
  fp12_cyclotomic_square_constant(&value)
}

fn fp12_cyclotomic_square_7_times(value: &Fp12Constant) -> Fp12Constant {
  let value = fp12_cyclotomic_square_6_times(value);
  fp12_cyclotomic_square_constant(&value)
}

fn fp12_cyclotomic_square_8_times(value: &Fp12Constant) -> Fp12Constant {
  let value = fp12_cyclotomic_square_7_times(value);
  fp12_cyclotomic_square_constant(&value)
}

fn fp12_cyclotomic_square_10_times(value: &Fp12Constant) -> Fp12Constant {
  let value = fp12_cyclotomic_square_8_times(value);
  let value = fp12_cyclotomic_square_constant(&value);
  fp12_cyclotomic_square_constant(&value)
}

fn fp12_cyclotomic_square_n_times(value: &Fp12Constant, square_count: u8) -> Fp12Constant {
  match square_count {
    6 => fp12_cyclotomic_square_6_times(value),
    7 => fp12_cyclotomic_square_7_times(value),
    8 => fp12_cyclotomic_square_8_times(value),
    10 => fp12_cyclotomic_square_10_times(value),
    _ => unreachable!("unsupported BN254 exp-by-x square block"),
  }
}

fn fp12_exp_by_x_window_constant(
  x17: &Fp12Constant,
  x25: &Fp12Constant,
  x29: &Fp12Constant,
  x39: &Fp12Constant,
  x41: &Fp12Constant,
  x43: &Fp12Constant,
  x49: &Fp12Constant,
  window: Bn254ExpByXWindow,
) -> Fp12Constant {
  match window {
    Bn254ExpByXWindow::X17 => *x17,
    Bn254ExpByXWindow::X25 => *x25,
    Bn254ExpByXWindow::X29 => *x29,
    Bn254ExpByXWindow::X39 => *x39,
    Bn254ExpByXWindow::X41 => *x41,
    Bn254ExpByXWindow::X43 => *x43,
    Bn254ExpByXWindow::X49 => *x49,
  }
}

pub(crate) fn fp12_exp_by_neg_x_constant(value: &Fp12Constant) -> Fp12Constant {
  // Compute value^x for the BN254 parameter
  // x = 0x44e992b44a6909f1 = 4965661367192848881.
  //
  // The shift-and-add recipe itself lives in `bn254/final_exp_chain.rs` so the
  // host/reference path and the circuit path cannot silently diverge. This
  // helper is only used from the hard part, so the repeated square blocks act
  // on cyclotomic-subgroup elements and can use cyclotomic square directly.
  debug_assert_eq!(BN254_X_ABS, 0x44e9_92b4_4a69_09f1);
  let x2 = fp12_cyclotomic_square_constant(value);
  let x4 = fp12_cyclotomic_square_constant(&x2);
  let x8 = fp12_cyclotomic_square_constant(&x4);
  let x16 = fp12_cyclotomic_square_constant(&x8);
  let x32 = fp12_cyclotomic_square_constant(&x16);

  let x10 = fp12_mul_constant(&x8, &x2);
  let x17 = fp12_mul_constant(&x16, value);
  let x25 = fp12_mul_constant(&x17, &x8);
  let x29 = fp12_mul_constant(&x25, &x4);
  let x39 = fp12_mul_constant(&x29, &x10);
  let x41 = fp12_mul_constant(&x25, &x16);
  let x43 = fp12_mul_constant(&x41, &x2);
  let x49 = fp12_mul_constant(&x32, &x17);

  let mut exp = fp12_exp_by_x_window_constant(
    &x17,
    &x25,
    &x29,
    &x39,
    &x41,
    &x43,
    &x49,
    BN254_EXP_BY_X_CHAIN_START,
  );

  for (square_count, window) in BN254_EXP_BY_X_CHAIN_STEPS {
    exp = fp12_cyclotomic_square_n_times(&exp, *square_count);
    exp = fp12_mul_constant(
      &exp,
      &fp12_exp_by_x_window_constant(&x17, &x25, &x29, &x39, &x41, &x43, &x49, *window),
    );
  }

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
  let y1 = fp12_cyclotomic_square_constant(&y0);
  let y2 = fp12_cyclotomic_square_constant(&y1);
  let mut y3 = fp12_mul_constant(&y2, &y1);
  let y4 = fp12_exp_by_neg_x_constant(&y3);
  let y5 = fp12_cyclotomic_square_constant(&y4);
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
  use crate::bn254::tests::{ark_bn254_miller_loop_accumulate, ark_to_midnight_fq12};
  use ark_bn254::{G1Affine as ArkG1Affine, G2Affine as ArkG2Affine};
  use ark_ec::AffineRepr;

  #[test]
  fn exp_by_neg_x_constant_matches_generic_square_and_multiply() {
    let miller_output =
      ark_bn254_miller_loop_accumulate(ArkG2Affine::generator(), ArkG1Affine::generator());
    let sample =
      bn254_final_exponentiation_easy_part_constant(&ark_to_midnight_fq12(&miller_output));
    let expected = fp12_conjugate_constant(&fp12_pow_constant_exp(&sample, &[BN254_X_ABS]));
    let actual = fp12_exp_by_neg_x_constant(&sample);

    assert_eq!(actual, expected);
  }
}
