//! Canonical non-native BN254 tower scaffolding for future R1CS lowering.
//!
//! This module mirrors the existing BN254 tower split used by the Halo2 /
//! Midnight implementation, but keeps the logic local to the canonical R1CS
//! lane. The current step ports the tower formulas and value structure; full
//! sound non-native R1CS constraints are still future work.

mod fq;
mod fq12;
mod fq2;
mod fq6;

pub use fq::{
  FqConstant, fq_add_constant, fq_mul_constant, fq_neg_constant, fq_square_constant,
  fq_sub_constant,
};
pub use fq2::{
  Fq2Constant, fq2_add_constant, fq2_frobenius_map_constant, fq2_inv_constant, fq2_mul_constant,
  fq2_neg_constant, fq2_square_constant, fq2_sub_constant,
};
pub use fq6::{
  Fq6Constant, fq6_add_constant, fq6_inv_constant, fq6_mul_by_nonresidue_constant,
  fq6_mul_constant, fq6_nonresidue_constant, fq6_square_constant, fq6_sub_constant,
};
pub use fq12::{
  Fq12Constant, bn254_final_exponentiation_constant, fq12_add_constant, fq12_conjugate_constant,
  fq12_cyclotomic_square_constant, fq12_frobenius_map_constant, fq12_inv_constant,
  fq12_mul_constant, fq12_nonresidue_constant, fq12_one_constant, fq12_square_constant,
  fq12_sub_constant,
};
