//! Halo2-facing circuit foundations.
//!
//! Week 1 now wires BN254 foreign-field and minimal G1 operations to real
//! Midnight/Halo2 chips, together with lightweight layout reporting. Week 2
//! and the current Week 3 slice add narrow BN254 Fp2/Fp6/Fp12 layers plus
//! minimal G2 affine/projective support, all organized under the `bn254/`
//! module. Pairings and verifier logic remain intentionally out of scope.
#![allow(clippy::multiple_crate_versions)]

use ff as _;
use halo2curves as _;

mod bn254;
pub mod metrics;
pub mod outer;
pub mod planning;

pub use bn254::{
  AssignedFp, AssignedFp2, AssignedFp6, AssignedFp12, AssignedG1, AssignedG1Point,
  AssignedG2Affine, AssignedG2LineCoeffs, AssignedG2MillerPoint, AssignedG2Projective,
  AssignedMillerAccumulator, Bn254EccChip, Bn254FpChip, Fp2AddCircuit, Fp2MulCircuit,
  Fp2SquareCircuit, Fp6AddCircuit, Fp6MulCircuit, Fp6SquareCircuit, Fp12AddCircuit, Fp12MulCircuit,
  Fp12SquareCircuit, FpAddCircuit, FpMulCircuit, G1AddCircuit, G1OnCurveCircuit,
  G2DoubleWithLineCircuit, G2MixedAddWithLineCircuit, G2NegCircuit, G2OnCurveCircuit,
  G2ProjectiveAddCircuit, G2ProjectiveDoubleCircuit, G2ProjectiveFromAffineCircuit,
  G2ProjectiveIdentityCircuit, G2ProjectiveNegCircuit, MillerAccumulatorMulByLineCircuit,
  MillerAccumulatorSquareCircuit, MillerLoopCircuit, MillerStep, MillerStepConstant, NativeField,
  PreparedG2Miller, fp_add_k, fp_add_layout_metrics, fp_mul_k, fp_mul_layout_metrics, fp2_add_k,
  fp2_add_layout_metrics, fp2_mul_k, fp2_mul_layout_metrics, fp2_square_k,
  fp2_square_layout_metrics, fp6_add_k, fp6_add_layout_metrics, fp6_mul_k, fp6_mul_layout_metrics,
  fp6_nonresidue, fp6_square_k, fp6_square_layout_metrics, fp12_add_k, fp12_add_layout_metrics,
  fp12_mul_k, fp12_mul_layout_metrics, fp12_nonresidue, fp12_square_k, fp12_square_layout_metrics,
  g1_add_k, g1_add_layout_metrics, g2_curve_coeff_b, g2_double_with_line_k,
  g2_double_with_line_layout_metrics, g2_mixed_add_with_line_k,
  g2_mixed_add_with_line_layout_metrics, g2_neg_k, g2_neg_layout_metrics, g2_on_curve_k,
  g2_on_curve_layout_metrics, g2_proj_add_k, g2_proj_add_layout_metrics, g2_proj_double_k,
  g2_proj_double_layout_metrics, g2_proj_from_affine_k, g2_proj_from_affine_layout_metrics,
  miller_accumulator_mul_by_line_k, miller_accumulator_mul_by_line_layout_metrics,
  miller_accumulator_square_k, miller_accumulator_square_layout_metrics, miller_loop,
  miller_loop_k, miller_loop_layout_metrics,
};
pub use metrics::{CostEstimate, LayoutMetrics};
pub use outer::{CircuitBuildStatus, OuterWrapperCircuit};
pub use planning::{
  CircuitPlanningView, PRIMITIVE_COUNT, PrimitiveCostEntry, PrimitiveCostLayer, PrimitiveCostTable,
  PrimitiveDefinition, primitive_definitions,
};
