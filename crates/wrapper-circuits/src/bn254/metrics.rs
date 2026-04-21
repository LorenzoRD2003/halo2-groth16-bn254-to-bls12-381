use midnight_circuits::midnight_proofs::{dev::cost_model::circuit_model, plonk::Circuit};

use crate::metrics::LayoutMetrics;

use super::{
  Fp2AddCircuit, Fp2MulCircuit, Fp2SquareCircuit, Fp6AddCircuit, Fp6MulCircuit, Fp6SquareCircuit,
  FpAddCircuit, FpMulCircuit, G1AddCircuit, G2NegCircuit, G2OnCurveCircuit, G2ProjectiveAddCircuit,
  G2ProjectiveDoubleCircuit, G2ProjectiveFromAffineCircuit, NativeField,
};

/// Models a circuit and returns real layout metrics.
#[must_use]
pub fn measure_layout(circuit: &impl Circuit<NativeField>) -> LayoutMetrics {
  LayoutMetrics::from(circuit_model::<NativeField, 48, 32>(circuit))
}

/// Real layout metrics for the current BN254 foreign-field addition circuit.
#[must_use]
pub fn fp_add_layout_metrics() -> LayoutMetrics {
  measure_layout(&FpAddCircuit::sample())
}

/// Real layout metrics for the current BN254 foreign-field multiplication circuit.
#[must_use]
pub fn fp_mul_layout_metrics() -> LayoutMetrics {
  measure_layout(&FpMulCircuit::sample())
}

/// Real layout metrics for the current BN254 Fp2 addition circuit.
#[must_use]
pub fn fp2_add_layout_metrics() -> LayoutMetrics {
  measure_layout(&Fp2AddCircuit::sample())
}

/// Real layout metrics for the current BN254 Fp2 multiplication circuit.
#[must_use]
pub fn fp2_mul_layout_metrics() -> LayoutMetrics {
  measure_layout(&Fp2MulCircuit::sample())
}

/// Real layout metrics for the current BN254 Fp2 square circuit.
#[must_use]
pub fn fp2_square_layout_metrics() -> LayoutMetrics {
  measure_layout(&Fp2SquareCircuit::sample())
}

/// Real layout metrics for the current BN254 Fp6 addition circuit.
#[must_use]
pub fn fp6_add_layout_metrics() -> LayoutMetrics {
  measure_layout(&Fp6AddCircuit::sample())
}

/// Real layout metrics for the current BN254 Fp6 multiplication circuit.
#[must_use]
pub fn fp6_mul_layout_metrics() -> LayoutMetrics {
  measure_layout(&Fp6MulCircuit::sample())
}

/// Real layout metrics for the current BN254 Fp6 square circuit.
#[must_use]
pub fn fp6_square_layout_metrics() -> LayoutMetrics {
  measure_layout(&Fp6SquareCircuit::sample())
}

/// Real layout metrics for the current BN254 G1 addition circuit.
#[must_use]
pub fn g1_add_layout_metrics() -> LayoutMetrics {
  measure_layout(&G1AddCircuit::sample())
}

/// Real layout metrics for the current BN254 G2 on-curve circuit.
#[must_use]
pub fn g2_on_curve_layout_metrics() -> LayoutMetrics {
  measure_layout(&G2OnCurveCircuit::sample())
}

/// Real layout metrics for the current BN254 G2 negation circuit.
#[must_use]
pub fn g2_neg_layout_metrics() -> LayoutMetrics {
  measure_layout(&G2NegCircuit::sample())
}

/// Real layout metrics for the current BN254 G2 affine-to-projective embedding circuit.
#[must_use]
pub fn g2_proj_from_affine_layout_metrics() -> LayoutMetrics {
  measure_layout(&G2ProjectiveFromAffineCircuit::sample())
}

/// Real layout metrics for the current BN254 G2 projective doubling circuit.
#[must_use]
pub fn g2_proj_double_layout_metrics() -> LayoutMetrics {
  measure_layout(&G2ProjectiveDoubleCircuit::sample())
}

/// Real layout metrics for the current BN254 G2 projective addition circuit.
#[must_use]
pub fn g2_proj_add_layout_metrics() -> LayoutMetrics {
  measure_layout(&G2ProjectiveAddCircuit::sample())
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp_add_k() -> u32 {
  fp_add_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp_mul_k() -> u32 {
  fp_mul_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp2_add_k() -> u32 {
  fp2_add_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp2_mul_k() -> u32 {
  fp2_mul_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp2_square_k() -> u32 {
  fp2_square_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp6_add_k() -> u32 {
  fp6_add_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp6_mul_k() -> u32 {
  fp6_mul_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp6_square_k() -> u32 {
  fp6_square_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn g1_add_k() -> u32 {
  g1_add_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn g2_on_curve_k() -> u32 {
  g2_on_curve_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn g2_neg_k() -> u32 {
  g2_neg_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn g2_proj_from_affine_k() -> u32 {
  g2_proj_from_affine_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn g2_proj_double_k() -> u32 {
  g2_proj_double_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn g2_proj_add_k() -> u32 {
  g2_proj_add_layout_metrics().k
}
