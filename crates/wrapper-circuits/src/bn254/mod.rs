//! Shared Midnight/Halo2 wiring for the BN254 primitive layer.

mod ecc;
mod field;
mod fp12;
mod fp2;
mod fp6;
mod g2;
mod host;
mod metrics;
mod traits;
mod types;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use ecc::{G1AddCircuit, G1OnCurveCircuit};
pub use field::{FpAddCircuit, FpMulCircuit};
pub use fp2::{AssignedFp2, Fp2AddCircuit, Fp2MulCircuit, Fp2SquareCircuit};
pub use fp6::{AssignedFp6, Fp6AddCircuit, Fp6MulCircuit, Fp6SquareCircuit, fp6_nonresidue};
pub use fp12::{AssignedFp12, Fp12AddCircuit, Fp12MulCircuit, Fp12SquareCircuit, fp12_nonresidue};
pub use g2::{
  AssignedG2Affine, AssignedG2LineCoeffs, AssignedG2MillerPoint, AssignedG2Projective,
  AssignedMillerAccumulator, G2DoubleWithLineCircuit, G2MixedAddWithLineCircuit, G2NegCircuit,
  G2OnCurveCircuit, G2ProjectiveAddCircuit, G2ProjectiveDoubleCircuit,
  G2ProjectiveFromAffineCircuit, G2ProjectiveIdentityCircuit, G2ProjectiveNegCircuit,
  g2_curve_coeff_b,
};
pub use metrics::{
  fp_add_k, fp_add_layout_metrics, fp_mul_k, fp_mul_layout_metrics, fp2_add_k,
  fp2_add_layout_metrics, fp2_mul_k, fp2_mul_layout_metrics, fp2_square_k,
  fp2_square_layout_metrics, fp6_add_k, fp6_add_layout_metrics, fp6_mul_k, fp6_mul_layout_metrics,
  fp6_square_k, fp6_square_layout_metrics, fp12_add_k, fp12_add_layout_metrics, fp12_mul_k,
  fp12_mul_layout_metrics, fp12_square_k, fp12_square_layout_metrics, g1_add_k,
  g1_add_layout_metrics, g2_double_with_line_k, g2_double_with_line_layout_metrics,
  g2_mixed_add_with_line_k, g2_mixed_add_with_line_layout_metrics, g2_neg_k, g2_neg_layout_metrics,
  g2_on_curve_k, g2_on_curve_layout_metrics, g2_proj_add_k, g2_proj_add_layout_metrics,
  g2_proj_double_k, g2_proj_double_layout_metrics, g2_proj_from_affine_k,
  g2_proj_from_affine_layout_metrics,
};
pub(crate) use traits::{
  AssignedCircuitValue, AssignedFieldExt, synthesize_binary_value_circuit,
  synthesize_unary_value_circuit,
};
pub use types::{
  AssignedFp, AssignedG1, Bn254EccChip, Bn254FieldChip, Bn254FieldConfig, Bn254FpChip, Bn254G1Chip,
  Bn254G1Config, ForeignCurve, ForeignField, NativeField,
};
