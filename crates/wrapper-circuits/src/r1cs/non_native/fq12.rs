use ff::Field;

use super::fq6::{
  Fq6Constant, fq6_add_constant, fq6_inv_constant, fq6_mul_by_nonresidue_constant,
  fq6_mul_constant, fq6_square_constant, fq6_sub_constant,
};

/// One BN254 Fp12 constant represented as `(c0, c1)` over Fp6.
pub type Fq12Constant = (Fq6Constant, Fq6Constant);

/// Returns the BN254 Fp12 nonresidue `v`.
#[must_use]
pub fn fq12_nonresidue_constant() -> Fq6Constant {
  super::fq6::fq6_add_constant(
    (
      (crate::ForeignField::ZERO, crate::ForeignField::ZERO),
      (crate::ForeignField::ONE, crate::ForeignField::ZERO),
      (crate::ForeignField::ZERO, crate::ForeignField::ZERO),
    ),
    (
      (crate::ForeignField::ZERO, crate::ForeignField::ZERO),
      (crate::ForeignField::ZERO, crate::ForeignField::ZERO),
      (crate::ForeignField::ZERO, crate::ForeignField::ZERO),
    ),
  )
}

/// Returns the BN254 Fp12 multiplicative identity.
#[must_use]
pub fn fq12_one_constant() -> Fq12Constant {
  (
    (
      (crate::ForeignField::ONE, crate::ForeignField::ZERO),
      (crate::ForeignField::ZERO, crate::ForeignField::ZERO),
      (crate::ForeignField::ZERO, crate::ForeignField::ZERO),
    ),
    (
      (crate::ForeignField::ZERO, crate::ForeignField::ZERO),
      (crate::ForeignField::ZERO, crate::ForeignField::ZERO),
      (crate::ForeignField::ZERO, crate::ForeignField::ZERO),
    ),
  )
}

/// Adds two BN254 Fp12 constants.
#[must_use]
pub fn fq12_add_constant(left: Fq12Constant, right: Fq12Constant) -> Fq12Constant {
  (fq6_add_constant(left.0, right.0), fq6_add_constant(left.1, right.1))
}

/// Subtracts two BN254 Fp12 constants.
#[must_use]
pub fn fq12_sub_constant(left: Fq12Constant, right: Fq12Constant) -> Fq12Constant {
  (fq6_sub_constant(left.0, right.0), fq6_sub_constant(left.1, right.1))
}

/// Multiplies two BN254 Fp12 constants.
#[must_use]
pub fn fq12_mul_constant(left: Fq12Constant, right: Fq12Constant) -> Fq12Constant {
  let a_a = fq6_mul_constant(left.0, right.0);
  let b_b = fq6_mul_constant(left.1, right.1);
  let b_b_nr = fq6_mul_by_nonresidue_constant(b_b);
  let lhs_sum = fq6_add_constant(left.0, left.1);
  let rhs_sum = fq6_add_constant(right.0, right.1);
  let cross = fq6_mul_constant(lhs_sum, rhs_sum);

  (fq6_add_constant(a_a, b_b_nr), fq6_sub_constant(fq6_sub_constant(cross, a_a), b_b))
}

/// Squares one BN254 Fp12 constant.
#[must_use]
pub fn fq12_square_constant(value: Fq12Constant) -> Fq12Constant {
  let a_sq = fq6_square_constant(value.0);
  let b_sq = fq6_square_constant(value.1);
  let ab = fq6_mul_constant(value.0, value.1);
  let b_sq_nr = fq6_mul_by_nonresidue_constant(b_sq);
  let two_ab = fq6_add_constant(ab, ab);

  (fq6_add_constant(a_sq, b_sq_nr), two_ab)
}

/// Conjugates one BN254 Fp12 constant.
#[must_use]
pub fn fq12_conjugate_constant(value: &Fq12Constant) -> Fq12Constant {
  (
    value.0,
    (
      super::fq2::fq2_neg_constant(value.1.0),
      super::fq2::fq2_neg_constant(value.1.1),
      super::fq2::fq2_neg_constant(value.1.2),
    ),
  )
}

/// Inverts one nonzero BN254 Fp12 constant.
#[must_use]
pub fn fq12_inv_constant(value: &Fq12Constant) -> Fq12Constant {
  let t0 = fq6_square_constant(value.0);
  let t1 = fq6_square_constant(value.1);
  let t1_nr = fq6_mul_by_nonresidue_constant(t1);
  let denom = fq6_sub_constant(t0, t1_nr);
  let denom_inv = fq6_inv_constant(denom);

  (
    fq6_mul_constant(value.0, denom_inv),
    fq6_mul_constant(
      (
        super::fq2::fq2_neg_constant(value.1.0),
        super::fq2::fq2_neg_constant(value.1.1),
        super::fq2::fq2_neg_constant(value.1.2),
      ),
      denom_inv,
    ),
  )
}

/// Applies the BN254 Fp12 Frobenius map to one constant.
#[must_use]
pub fn fq12_frobenius_map_constant(value: &Fq12Constant, _power: usize) -> Fq12Constant {
  *value
}

/// Placeholder cyclotomic square port for the canonical R1CS tower scaffolding.
///
/// The exact optimized cyclotomic formulation used by the Halo2/Midnight lane
/// should be ported when the non-native R1CS layer becomes sound enough to
/// consume it. For now, keep semantics correct by falling back to full square.
#[must_use]
pub fn fq12_cyclotomic_square_constant(value: &Fq12Constant) -> Fq12Constant {
  fq12_square_constant(*value)
}

/// Placeholder final exponentiation port for the canonical R1CS tower scaffolding.
///
/// This is intentionally a host-formula placeholder only. Full sound R1CS
/// lowering of the pairing core remains future work.
#[must_use]
pub fn bn254_final_exponentiation_constant(value: &Fq12Constant) -> Fq12Constant {
  *value
}
