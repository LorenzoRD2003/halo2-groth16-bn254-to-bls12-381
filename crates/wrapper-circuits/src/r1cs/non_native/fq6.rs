use ff::{Field, PrimeField};

use crate::ForeignField;

use super::fq2::{
  Fq2Constant, fq2_add_constant, fq2_inv_constant, fq2_mul_constant, fq2_square_constant,
  fq2_sub_constant,
};

/// One BN254 Fp6 constant represented as `(c0, c1, c2)` over Fp2.
pub type Fq6Constant = (Fq2Constant, Fq2Constant, Fq2Constant);

/// Returns the BN254 Fp6 nonresidue `9 + u`.
#[must_use]
pub fn fq6_nonresidue_constant() -> Fq2Constant {
  (
    ForeignField::from_str_vartime("9").expect("hard-coded BN254 Fp6 nonresidue c0 should parse"),
    ForeignField::ONE,
  )
}

fn fq2_mul_by_fp6_nonresidue_constant(value: Fq2Constant) -> Fq2Constant {
  let nine_c0 = value.0 * ForeignField::from(9_u64);
  let nine_c1 = value.1 * ForeignField::from(9_u64);
  (nine_c0 - value.1, nine_c1 + value.0)
}

/// Adds two BN254 Fp6 constants.
#[must_use]
pub fn fq6_add_constant(left: Fq6Constant, right: Fq6Constant) -> Fq6Constant {
  (
    fq2_add_constant(left.0, right.0),
    fq2_add_constant(left.1, right.1),
    fq2_add_constant(left.2, right.2),
  )
}

/// Subtracts two BN254 Fp6 constants.
#[must_use]
pub fn fq6_sub_constant(left: Fq6Constant, right: Fq6Constant) -> Fq6Constant {
  (
    fq2_sub_constant(left.0, right.0),
    fq2_sub_constant(left.1, right.1),
    fq2_sub_constant(left.2, right.2),
  )
}

/// Multiplies one BN254 Fp6 constant by the cubic nonresidue.
#[must_use]
pub fn fq6_mul_by_nonresidue_constant(value: Fq6Constant) -> Fq6Constant {
  (fq2_mul_by_fp6_nonresidue_constant(value.2), value.0, value.1)
}

/// Multiplies two BN254 Fp6 constants.
#[must_use]
pub fn fq6_mul_constant(left: Fq6Constant, right: Fq6Constant) -> Fq6Constant {
  let a_a = fq2_mul_constant(left.0, right.0);
  let b_b = fq2_mul_constant(left.1, right.1);
  let c_c = fq2_mul_constant(left.2, right.2);

  let t1 = fq2_sub_constant(
    fq2_mul_constant(fq2_add_constant(right.1, right.2), fq2_add_constant(left.1, left.2)),
    fq2_add_constant(c_c, b_b),
  );
  let t1 = fq2_add_constant(a_a, fq2_mul_by_fp6_nonresidue_constant(t1));

  let t3 = fq2_sub_constant(
    fq2_mul_constant(fq2_add_constant(right.0, right.2), fq2_add_constant(left.0, left.2)),
    fq2_sub_constant(fq2_add_constant(a_a, c_c), b_b),
  );

  let t2 = fq2_sub_constant(
    fq2_mul_constant(fq2_add_constant(right.0, right.1), fq2_add_constant(left.0, left.1)),
    fq2_add_constant(a_a, b_b),
  );
  let t2 = fq2_add_constant(t2, fq2_mul_by_fp6_nonresidue_constant(c_c));

  (t1, t2, t3)
}

/// Squares one BN254 Fp6 constant.
#[must_use]
pub fn fq6_square_constant(value: Fq6Constant) -> Fq6Constant {
  let s0 = fq2_square_constant(value.0);
  let s1 = fq2_add_constant(fq2_mul_constant(value.0, value.1), fq2_mul_constant(value.0, value.1));
  let s2 = fq2_square_constant(fq2_add_constant(fq2_sub_constant(value.0, value.1), value.2));
  let s3 = fq2_add_constant(fq2_mul_constant(value.1, value.2), fq2_mul_constant(value.1, value.2));
  let s4 = fq2_square_constant(value.2);

  (
    fq2_add_constant(fq2_mul_by_fp6_nonresidue_constant(s3), s0),
    fq2_add_constant(fq2_mul_by_fp6_nonresidue_constant(s4), s1),
    fq2_sub_constant(fq2_sub_constant(fq2_add_constant(fq2_add_constant(s1, s2), s3), s0), s4),
  )
}

/// Inverts one nonzero BN254 Fp6 constant.
#[must_use]
pub fn fq6_inv_constant(value: Fq6Constant) -> Fq6Constant {
  let t0 = fq2_sub_constant(
    fq2_square_constant(value.0),
    fq2_mul_by_fp6_nonresidue_constant(fq2_mul_constant(value.1, value.2)),
  );
  let t1 = fq2_sub_constant(
    fq2_mul_by_fp6_nonresidue_constant(fq2_square_constant(value.2)),
    fq2_mul_constant(value.0, value.1),
  );
  let t2 = fq2_sub_constant(fq2_square_constant(value.1), fq2_mul_constant(value.0, value.2));

  let denom = fq2_add_constant(
    fq2_mul_constant(value.0, t0),
    fq2_mul_by_fp6_nonresidue_constant(fq2_add_constant(
      fq2_mul_constant(value.2, t1),
      fq2_mul_constant(value.1, t2),
    )),
  );
  let denom_inv = fq2_inv_constant(denom);

  (
    fq2_mul_constant(t0, denom_inv),
    fq2_mul_constant(t1, denom_inv),
    fq2_mul_constant(t2, denom_inv),
  )
}
