//! Shared Midnight/Halo2 wiring for the BN254 primitive layer.

mod ecc;
mod field;
mod fp2;
mod g2;
mod metrics;
mod types;

#[cfg(test)]
mod tests;

pub use ecc::{G1AddCircuit, G1OnCurveCircuit};
pub use field::{FpAddCircuit, FpMulCircuit};
pub use fp2::{AssignedFp2, Fp2AddCircuit, Fp2MulCircuit, Fp2SquareCircuit};
pub use g2::{AssignedG2Affine, G2NegCircuit, G2OnCurveCircuit, g2_curve_coeff_b};
pub use metrics::{
  fp_add_k, fp_add_layout_metrics, fp_mul_k, fp_mul_layout_metrics, fp2_add_k,
  fp2_add_layout_metrics, fp2_mul_k, fp2_mul_layout_metrics, fp2_square_k,
  fp2_square_layout_metrics, g1_add_k, g1_add_layout_metrics, g2_neg_k, g2_neg_layout_metrics,
  g2_on_curve_k, g2_on_curve_layout_metrics,
};
pub use types::{
  AssignedFp, AssignedG1, Bn254EccChip, Bn254FieldChip, Bn254FieldConfig, Bn254FpChip, Bn254G1Chip,
  Bn254G1Config, ForeignCurve, ForeignField, NativeField,
};
