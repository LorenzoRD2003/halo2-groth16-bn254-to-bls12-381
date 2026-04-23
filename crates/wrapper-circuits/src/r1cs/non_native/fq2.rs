use ff::Field;

use crate::ForeignField;

/// One BN254 Fp2 constant represented as `c0 + c1 * u`.
pub type Fq2Constant = (ForeignField, ForeignField);

/// Adds two BN254 Fp2 constants.
#[must_use]
pub fn fq2_add_constant(left: Fq2Constant, right: Fq2Constant) -> Fq2Constant {
  (left.0 + right.0, left.1 + right.1)
}

/// Subtracts two BN254 Fp2 constants.
#[must_use]
pub fn fq2_sub_constant(left: Fq2Constant, right: Fq2Constant) -> Fq2Constant {
  (left.0 - right.0, left.1 - right.1)
}

/// Negates one BN254 Fp2 constant.
#[must_use]
pub fn fq2_neg_constant(value: Fq2Constant) -> Fq2Constant {
  (-value.0, -value.1)
}

/// Multiplies two BN254 Fp2 constants assuming `u^2 = -1`.
#[must_use]
pub fn fq2_mul_constant(left: Fq2Constant, right: Fq2Constant) -> Fq2Constant {
  let ac = left.0 * right.0;
  let bd = left.1 * right.1;
  let ad = left.0 * right.1;
  let bc = left.1 * right.0;

  (ac - bd, ad + bc)
}

/// Squares one BN254 Fp2 constant assuming `u^2 = -1`.
#[must_use]
pub fn fq2_square_constant(value: Fq2Constant) -> Fq2Constant {
  let a_sq = value.0.square();
  let b_sq = value.1.square();
  let ab = value.0 * value.1;

  (a_sq - b_sq, ab + ab)
}

/// Inverts one nonzero BN254 Fp2 constant.
#[must_use]
pub fn fq2_inv_constant(value: Fq2Constant) -> Fq2Constant {
  let norm = value.0.square() + value.1.square();
  let norm_inv = norm.invert().expect("nonzero Fp2 norm should be invertible");
  (value.0 * norm_inv, -value.1 * norm_inv)
}

/// Applies the BN254 Fp2 Frobenius map to one constant.
#[must_use]
pub fn fq2_frobenius_map_constant(value: Fq2Constant, power: usize) -> Fq2Constant {
  if power % 2 == 0 { value } else { (value.0, -value.1) }
}
