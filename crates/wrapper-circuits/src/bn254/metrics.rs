use midnight_circuits::midnight_proofs::{dev::cost_model::circuit_model, plonk::Circuit};

use crate::metrics::LayoutMetrics;

use super::{
  Fp2AddCircuit, Fp2MulCircuit, Fp2SquareCircuit, FpAddCircuit, FpMulCircuit, G1AddCircuit,
  NativeField,
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

/// Real layout metrics for the current BN254 G1 addition circuit.
#[must_use]
pub fn g1_add_layout_metrics() -> LayoutMetrics {
  measure_layout(&G1AddCircuit::sample())
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
pub fn g1_add_k() -> u32 {
  g1_add_layout_metrics().k
}
