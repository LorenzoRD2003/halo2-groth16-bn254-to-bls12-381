use super::*;
use crate::bn254::{
  BN254_EXP_BY_X_CHAIN_START, BN254_EXP_BY_X_CHAIN_STEPS, BN254_X_ABS, Bn254ExpByXWindow,
  Bn254ExpByXWindowSign,
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

fn fp12_cyclotomic_square_n_times(value: &Fp12Constant, square_count: u8) -> Fp12Constant {
  let mut squared = *value;
  for _ in 0..square_count {
    squared = fp12_cyclotomic_square_constant(&squared);
  }
  squared
}

fn fp12_exp_by_x_window_constant(
  x17: &Fp12Constant,
  x35: &Fp12Constant,
  x37: &Fp12Constant,
  x79: &Fp12Constant,
  x83: &Fp12Constant,
  x101: &Fp12Constant,
  x105: &Fp12Constant,
  window: Bn254ExpByXWindow,
) -> Fp12Constant {
  match window {
    Bn254ExpByXWindow::X17 => *x17,
    Bn254ExpByXWindow::X35 => *x35,
    Bn254ExpByXWindow::X37 => *x37,
    Bn254ExpByXWindow::X79 => *x79,
    Bn254ExpByXWindow::X83 => *x83,
    Bn254ExpByXWindow::X101 => *x101,
    Bn254ExpByXWindow::X105 => *x105,
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
  let x64 = fp12_cyclotomic_square_constant(&x32);

  let x17 = fp12_mul_constant(&x16, value);
  let x19 = fp12_mul_constant(&x17, &x2);
  let x35 = fp12_mul_constant(&x19, &x16);
  let x37 = fp12_mul_constant(&x35, &x2);
  let x83 = fp12_mul_constant(&x19, &x64);
  let x79 = fp12_mul_constant(&x83, &fp12_conjugate_constant(&x4));
  let x101 = fp12_mul_constant(&x37, &x64);
  let x105 = fp12_mul_constant(&x101, &x4);

  let mut exp = fp12_exp_by_x_window_constant(
    &x17,
    &x35,
    &x37,
    &x79,
    &x83,
    &x101,
    &x105,
    BN254_EXP_BY_X_CHAIN_START,
  );

  for step in BN254_EXP_BY_X_CHAIN_STEPS {
    exp = fp12_cyclotomic_square_n_times(&exp, step.square_count);
    let window_value =
      fp12_exp_by_x_window_constant(&x17, &x35, &x37, &x79, &x83, &x101, &x105, step.window);
    exp = match step.sign {
      Bn254ExpByXWindowSign::Positive => fp12_mul_constant(&exp, &window_value),
      Bn254ExpByXWindowSign::Negative => {
        fp12_mul_constant(&exp, &fp12_conjugate_constant(&window_value))
      }
    };
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
