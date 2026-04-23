use crate::ForeignField;

/// One BN254 base-field constant carried by the canonical R1CS non-native tower.
pub type FqConstant = ForeignField;

/// Adds two BN254 base-field constants.
#[must_use]
pub fn fq_add_constant(left: FqConstant, right: FqConstant) -> FqConstant {
  left + right
}

/// Subtracts two BN254 base-field constants.
#[must_use]
pub fn fq_sub_constant(left: FqConstant, right: FqConstant) -> FqConstant {
  left - right
}

/// Negates one BN254 base-field constant.
#[must_use]
pub fn fq_neg_constant(value: FqConstant) -> FqConstant {
  -value
}

/// Multiplies two BN254 base-field constants.
#[must_use]
pub fn fq_mul_constant(left: FqConstant, right: FqConstant) -> FqConstant {
  left * right
}

/// Squares one BN254 base-field constant.
#[must_use]
pub fn fq_square_constant(value: FqConstant) -> FqConstant {
  value.square()
}
