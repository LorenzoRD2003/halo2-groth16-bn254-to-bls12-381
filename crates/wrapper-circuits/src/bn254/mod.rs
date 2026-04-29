//! Shared Midnight/Halo2 wiring for the BN254 primitive layer.

mod ecc;
mod field;
mod final_exp_chain;
mod fp12;
mod fp2;
mod fp6;
mod g2;
mod host;
mod metrics;
mod traits;
mod types;

#[cfg(test)]
mod tests;

pub use ecc::{G1AddCircuit, G1OnCurveCircuit};
pub use field::{FpAddCircuit, FpMulCircuit};
pub(crate) use final_exp_chain::{
  BN254_EXP_BY_X_CHAIN_START, BN254_EXP_BY_X_CHAIN_STEPS, BN254_X_ABS, Bn254ExpByXWindow,
  Bn254ExpByXWindowSign,
};
pub use final_exp_chain::{
  Bn254ExpByXChainCandidate, Bn254ExpByXChainSearchConfig, Bn254ExpByXChainSearchWeights,
  retained_bn254_exp_by_x_chain_candidate, search_bn254_exp_by_x_candidates,
  search_bn254_exp_by_x_candidates_with_windows,
};
pub use fp2::{AssignedFp2, Fp2AddCircuit, Fp2MulCircuit, Fp2SquareCircuit};
pub use fp6::{AssignedFp6, Fp6AddCircuit, Fp6MulCircuit, Fp6SquareCircuit, fp6_nonresidue};
pub use fp12::{
  AssignedFp12, Fp12AddCircuit, Fp12CompressedCyclotomicSquareBlockCircuit,
  Fp12CyclotomicSquareCircuit, Fp12MulByUnitaryInverseCircuit, Fp12MulCircuit, Fp12SquareCircuit,
  fp12_nonresidue,
};
pub(crate) use g2::pairing_check_with_prepared_terms_on_host;
pub use g2::{
  AssignedG1Point, AssignedG2Affine, AssignedG2LineCoeffs, AssignedG2MillerPoint,
  AssignedG2Projective, AssignedMillerAccumulator, Bn254MillerAddend, Bn254MillerSchedule,
  Bn254MillerScheduleStep, FinalExponentiationCircuit, FinalExponentiationEasyPartCircuit,
  FinalExponentiationHardPartCircuit, G2DoubleWithLineCircuit, G2MixedAddWithLineCircuit,
  G2NegCircuit, G2OnCurveCircuit, G2ProjectiveAddCircuit, G2ProjectiveDoubleCircuit,
  G2ProjectiveFromAffineCircuit, G2ProjectiveIdentityCircuit, G2ProjectiveNegCircuit,
  MillerAccumulatorMulByLineCircuit, MillerAccumulatorMulByLineSparseCircuit,
  MillerAccumulatorSquareCircuit, MillerLoopCircuit, MillerStep, MillerStepConstant,
  PairingCheckCircuit, PairingFinalExponentiationCircuit, PreparedConstantG2Miller,
  PreparedG2Miller, bn254_ate_loop_count, final_exponentiation, g2_curve_coeff_b, miller_loop,
  multi_miller_loop, pairing_check,
};
pub(crate) use metrics::{measure_layout, measure_layout_with_v1};
pub use metrics::{
  final_exponentiation_easy_part_k, final_exponentiation_easy_part_layout_metrics,
  final_exponentiation_easy_part_layout_metrics_v1, final_exponentiation_hard_part_k,
  final_exponentiation_hard_part_layout_metrics, final_exponentiation_hard_part_layout_metrics_v1,
  final_exponentiation_k, final_exponentiation_layout_metrics, final_exponentiation_layout_metrics_v1,
  fp_add_k, fp_add_layout_metrics, fp_mul_k, fp_mul_layout_metrics, fp2_add_k,
  fp2_add_layout_metrics, fp2_mul_k, fp2_mul_layout_metrics, fp2_square_k,
  fp2_square_layout_metrics, fp6_add_k, fp6_add_layout_metrics, fp6_mul_k, fp6_mul_layout_metrics,
  fp6_square_k, fp6_square_layout_metrics, fp12_add_k, fp12_add_layout_metrics,
  fp12_compressed_cyclotomic_square_block_layout_metrics, fp12_cyclotomic_square_k,
  fp12_cyclotomic_square_layout_metrics, fp12_mul_by_unitary_inverse_layout_metrics, fp12_mul_k,
  fp12_mul_layout_metrics, fp12_square_k, fp12_square_layout_metrics, g1_add_k,
  g1_add_layout_metrics, g2_double_with_line_k, g2_double_with_line_layout_metrics,
  g2_mixed_add_with_line_k, g2_mixed_add_with_line_layout_metrics, g2_neg_k,
  g2_neg_layout_metrics, g2_on_curve_k, g2_on_curve_layout_metrics, g2_proj_add_k,
  g2_proj_add_layout_metrics, g2_proj_double_k, g2_proj_double_layout_metrics,
  g2_proj_from_affine_k, g2_proj_from_affine_layout_metrics, miller_accumulator_mul_by_line_k,
  miller_accumulator_mul_by_line_layout_metrics, miller_accumulator_mul_by_line_sparse_k,
  miller_accumulator_mul_by_line_sparse_layout_metrics, miller_accumulator_square_k,
  miller_accumulator_square_layout_metrics, miller_loop_k, miller_loop_layout_metrics,
  miller_loop_layout_metrics_v1, pairing_check_k, pairing_check_layout_metrics,
  pairing_check_layout_metrics_v1,
};
pub(crate) use traits::{
  AssignedCircuitValue, AssignedFieldExt, synthesize_binary_value_circuit,
  synthesize_unary_value_circuit,
};
pub use types::{
  AssignedBool, AssignedFp, AssignedG1, Bls12HostField, Bn254BitChip, Bn254BoolChip,
  Bn254BoolConfig, Bn254EccChip, Bn254FieldChip, Bn254FieldConfig, Bn254FpChip, Bn254G1Chip,
  Bn254G1Config, ForeignCurve, ForeignField, NativeField,
};
