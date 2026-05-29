use std::marker::PhantomData;

use ff::Field;
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, floor_planner::V1},
  dev::cost_model::circuit_model,
  plonk::{Circuit, ConstraintSystem, Error, FloorPlanner},
};

use crate::metrics::LayoutMetrics;

use super::{
  FinalExponentiationCircuit, FinalExponentiationEasyPartCircuit,
  FinalExponentiationHardPartCircuit, Fp2AddCircuit, Fp2MulCircuit, Fp2SquareCircuit,
  Fp6AddCircuit, Fp6MulCircuit, Fp6SquareCircuit, Fp12AddCircuit,
  Fp12CompressedCyclotomicSquareBlockCircuit, Fp12CyclotomicSquareCircuit,
  Fp12MulByUnitaryInverseCircuit, Fp12MulCircuit, Fp12SquareCircuit, FpAddCircuit, FpMulCircuit,
  G1AddCircuit, G2DoubleWithLineCircuit, G2MixedAddWithLineCircuit, G2NegCircuit, G2OnCurveCircuit,
  G2ProjectiveAddCircuit, G2ProjectiveDoubleCircuit, G2ProjectiveFromAffineCircuit,
  MillerAccumulatorMulByLineCircuit, MillerAccumulatorMulByLineSparseCircuit,
  MillerAccumulatorSquareCircuit, MillerLoopCircuit, NativeField, PairingCheckCircuit,
};

#[derive(Clone, Debug)]
struct FloorPlannerOverride<C, P> {
  inner: C,
  _planner: PhantomData<P>,
}

impl<C, P> FloorPlannerOverride<C, P> {
  fn new(inner: C) -> Self {
    Self { inner, _planner: PhantomData }
  }
}

impl<F, C, P> Circuit<F> for FloorPlannerOverride<C, P>
where
  F: Field,
  C: Circuit<F> + Clone,
  P: FloorPlanner,
{
  type Config = C::Config;
  type FloorPlanner = P;
  type Params = C::Params;

  fn without_witnesses(&self) -> Self {
    Self::new(self.inner.without_witnesses())
  }

  fn params(&self) -> Self::Params {
    self.inner.params()
  }

  fn configure_with_params(meta: &mut ConstraintSystem<F>, params: Self::Params) -> Self::Config {
    C::configure_with_params(meta, params)
  }

  fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
    C::configure(meta)
  }

  fn synthesize(&self, config: Self::Config, layouter: impl Layouter<F>) -> Result<(), Error> {
    self.inner.synthesize(config, layouter)
  }
}

/// Models a circuit and returns real layout metrics.
#[must_use]
pub fn measure_layout(circuit: &impl Circuit<NativeField>) -> LayoutMetrics {
  LayoutMetrics::from(circuit_model::<NativeField, 48, 32>(circuit))
}

/// Models one native-field circuit using the V1 floor planner instead of the
/// circuit's default planner.
#[must_use]
pub(crate) fn measure_layout_with_v1<C>(circuit: &C) -> LayoutMetrics
where
  C: Circuit<NativeField> + Clone,
{
  let wrapped = FloorPlannerOverride::<C, V1>::new(circuit.clone());
  LayoutMetrics::from(circuit_model::<NativeField, 48, 32>(&wrapped))
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

/// Real layout metrics for the current BN254 Fp12 addition circuit.
#[must_use]
pub fn fp12_add_layout_metrics() -> LayoutMetrics {
  measure_layout(&Fp12AddCircuit::sample())
}

/// Real layout metrics for the current BN254 Fp12 multiplication circuit.
#[must_use]
pub fn fp12_mul_layout_metrics() -> LayoutMetrics {
  measure_layout(&Fp12MulCircuit::sample())
}

/// Real layout metrics for multiplying one cyclotomic element by the unitary
/// inverse of another.
#[must_use]
pub fn fp12_mul_by_unitary_inverse_layout_metrics() -> LayoutMetrics {
  measure_layout(&Fp12MulByUnitaryInverseCircuit::sample())
}

/// Real layout metrics for the current BN254 Fp12 square circuit.
#[must_use]
pub fn fp12_square_layout_metrics() -> LayoutMetrics {
  measure_layout(&Fp12SquareCircuit::sample())
}

/// Real layout metrics for the current BN254 Fp12 cyclotomic-square circuit.
#[must_use]
pub fn fp12_cyclotomic_square_layout_metrics() -> LayoutMetrics {
  measure_layout(&Fp12CyclotomicSquareCircuit::sample())
}

/// Real layout metrics for one compressed cyclotomic square block of the given
/// length.
#[must_use]
pub fn fp12_compressed_cyclotomic_square_block_layout_metrics(square_count: u8) -> LayoutMetrics {
  measure_layout(&Fp12CompressedCyclotomicSquareBlockCircuit::sample(square_count))
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

/// Real layout metrics for the current BN254 G2 Miller-path doubling-with-line circuit.
#[must_use]
pub fn g2_double_with_line_layout_metrics() -> LayoutMetrics {
  measure_layout(&G2DoubleWithLineCircuit::sample())
}

/// Real layout metrics for the current BN254 G2 Miller-path mixed-add-with-line circuit.
#[must_use]
pub fn g2_mixed_add_with_line_layout_metrics() -> LayoutMetrics {
  measure_layout(&G2MixedAddWithLineCircuit::sample())
}

/// Real layout metrics for the current BN254 Miller-accumulator square circuit.
#[must_use]
pub fn miller_accumulator_square_layout_metrics() -> LayoutMetrics {
  measure_layout(&MillerAccumulatorSquareCircuit::sample())
}

/// Real layout metrics for the current BN254 Miller-accumulator mul-by-line circuit.
#[must_use]
pub fn miller_accumulator_mul_by_line_layout_metrics() -> LayoutMetrics {
  measure_layout(&MillerAccumulatorMulByLineCircuit::sample())
}

/// Real layout metrics for the current optimized BN254 Miller-accumulator sparse mul-by-line circuit.
#[must_use]
pub fn miller_accumulator_mul_by_line_sparse_layout_metrics() -> LayoutMetrics {
  measure_layout(&MillerAccumulatorMulByLineSparseCircuit::sample())
}

/// Real layout metrics for the current narrow BN254 Miller-loop circuit.
#[must_use]
pub fn miller_loop_layout_metrics() -> LayoutMetrics {
  measure_layout(&MillerLoopCircuit::sample())
}

/// Real layout metrics for the current narrow BN254 Miller-loop circuit under
/// the V1 floor planner.
#[must_use]
pub fn miller_loop_layout_metrics_v1() -> LayoutMetrics {
  measure_layout_with_v1(&MillerLoopCircuit::sample())
}

/// Real layout metrics for the current narrow BN254 final exponentiation circuit.
#[must_use]
pub fn final_exponentiation_layout_metrics() -> LayoutMetrics {
  measure_layout(&FinalExponentiationCircuit::sample())
}

/// Real layout metrics for the current narrow BN254 final-exponentiation
/// circuit under the V1 floor planner.
#[must_use]
pub fn final_exponentiation_layout_metrics_v1() -> LayoutMetrics {
  measure_layout_with_v1(&FinalExponentiationCircuit::sample())
}

/// Real layout metrics for the current narrow BN254 final exponentiation easy-part circuit.
#[must_use]
pub fn final_exponentiation_easy_part_layout_metrics() -> LayoutMetrics {
  measure_layout(&FinalExponentiationEasyPartCircuit::sample())
}

/// Real layout metrics for the current narrow BN254 final-exponentiation
/// easy-part circuit under the V1 floor planner.
#[must_use]
pub fn final_exponentiation_easy_part_layout_metrics_v1() -> LayoutMetrics {
  measure_layout_with_v1(&FinalExponentiationEasyPartCircuit::sample())
}

/// Real layout metrics for the current narrow BN254 final exponentiation hard-part circuit.
#[must_use]
pub fn final_exponentiation_hard_part_layout_metrics() -> LayoutMetrics {
  measure_layout(&FinalExponentiationHardPartCircuit::sample())
}

/// Real layout metrics for the current narrow BN254 final-exponentiation
/// hard-part circuit under the V1 floor planner.
#[must_use]
pub fn final_exponentiation_hard_part_layout_metrics_v1() -> LayoutMetrics {
  measure_layout_with_v1(&FinalExponentiationHardPartCircuit::sample())
}

/// Real layout metrics for the current narrow BN254 pairing-check circuit.
#[must_use]
pub fn pairing_check_layout_metrics() -> LayoutMetrics {
  measure_layout(&PairingCheckCircuit::sample())
}

/// Real layout metrics for the current narrow BN254 pairing-check circuit
/// under the V1 floor planner.
#[must_use]
pub fn pairing_check_layout_metrics_v1() -> LayoutMetrics {
  measure_layout_with_v1(&PairingCheckCircuit::sample())
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
pub fn fp12_add_k() -> u32 {
  fp12_add_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp12_mul_k() -> u32 {
  fp12_mul_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp12_square_k() -> u32 {
  fp12_square_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp12_cyclotomic_square_k() -> u32 {
  fp12_cyclotomic_square_layout_metrics().k
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

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn g2_double_with_line_k() -> u32 {
  g2_double_with_line_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn g2_mixed_add_with_line_k() -> u32 {
  g2_mixed_add_with_line_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn miller_accumulator_square_k() -> u32 {
  miller_accumulator_square_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn miller_accumulator_mul_by_line_k() -> u32 {
  miller_accumulator_mul_by_line_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn miller_accumulator_mul_by_line_sparse_k() -> u32 {
  miller_accumulator_mul_by_line_sparse_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn miller_loop_k() -> u32 {
  miller_loop_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn final_exponentiation_k() -> u32 {
  final_exponentiation_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn final_exponentiation_easy_part_k() -> u32 {
  final_exponentiation_easy_part_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn final_exponentiation_hard_part_k() -> u32 {
  final_exponentiation_hard_part_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn pairing_check_k() -> u32 {
  pairing_check_layout_metrics().k
}
