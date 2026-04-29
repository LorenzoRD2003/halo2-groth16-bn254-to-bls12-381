//! Halo2-facing circuit foundations.
//!
//! Week 1 now wires BN254 foreign-field and minimal G1 operations to real
//! Midnight/Halo2 chips, together with lightweight layout reporting. Week 2
//! and the current Week 3 slice add narrow BN254 Fp2/Fp6/Fp12 layers plus
//! minimal G2 affine/projective support, all organized under the `bn254/`
//! module. Week 4 added the pairing core, and Week 5 now layers a first narrow
//! Groth16 BN254 verifier slice on top: real proof/VK consumption, IC linear
//! combination, and verifier reduction to one pairing-product check.
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::redundant_feature_names)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::double_must_use)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::large_types_passed_by_value)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::manual_repeat_n)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::type_complexity)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::unused_self)]
#![allow(clippy::wildcard_imports)]

use ff as _;
use halo2curves as _;

mod bn254;
mod groth16;
pub mod metrics;
pub mod outer;
pub mod planning;
pub mod r1cs;
mod test_support;

pub use bn254::{
  AssignedBool, AssignedFp, AssignedFp2, AssignedFp6, AssignedFp12, AssignedG1, AssignedG1Point,
  AssignedG2Affine, AssignedG2LineCoeffs, AssignedG2MillerPoint, AssignedG2Projective,
  AssignedMillerAccumulator, Bls12HostField, Bn254BitChip, Bn254BoolChip, Bn254BoolConfig,
  Bn254EccChip, Bn254ExpByXChainCandidate, Bn254ExpByXChainSearchConfig,
  Bn254ExpByXChainSearchWeights, Bn254FpChip, Bn254MillerAddend, Bn254MillerSchedule,
  Bn254MillerScheduleStep, FinalExponentiationCircuit, FinalExponentiationEasyPartCircuit,
  FinalExponentiationHardPartCircuit, ForeignField, Fp2AddCircuit, Fp2MulCircuit, Fp2SquareCircuit,
  Fp6AddCircuit, Fp6MulCircuit, Fp6SquareCircuit, Fp12AddCircuit,
  Fp12CompressedCyclotomicSquareBlockCircuit, Fp12CyclotomicSquareCircuit,
  Fp12MulByUnitaryInverseCircuit, Fp12MulCircuit, Fp12SquareCircuit, FpAddCircuit, FpMulCircuit,
  G1AddCircuit, G1OnCurveCircuit,
  G2DoubleWithLineCircuit, G2MixedAddWithLineCircuit, G2NegCircuit, G2OnCurveCircuit,
  G2ProjectiveAddCircuit, G2ProjectiveDoubleCircuit, G2ProjectiveFromAffineCircuit,
  G2ProjectiveIdentityCircuit, G2ProjectiveNegCircuit, MillerAccumulatorMulByLineCircuit,
  MillerAccumulatorMulByLineSparseCircuit, MillerAccumulatorSquareCircuit, MillerLoopCircuit,
  MillerStep, MillerStepConstant, NativeField, PairingCheckCircuit,
  PairingFinalExponentiationCircuit, PreparedG2Miller, bn254_ate_loop_count, final_exponentiation,
  final_exponentiation_easy_part_k, final_exponentiation_easy_part_layout_metrics,
  final_exponentiation_hard_part_k, final_exponentiation_hard_part_layout_metrics,
  final_exponentiation_k, final_exponentiation_layout_metrics, fp_add_k, fp_add_layout_metrics,
  fp_mul_k, fp_mul_layout_metrics, fp2_add_k, fp2_add_layout_metrics, fp2_mul_k,
  fp2_mul_layout_metrics, fp2_square_k, fp2_square_layout_metrics, fp6_add_k,
  fp6_add_layout_metrics, fp6_mul_k, fp6_mul_layout_metrics, fp6_nonresidue, fp6_square_k,
  fp6_square_layout_metrics, fp12_add_k, fp12_add_layout_metrics,
  fp12_compressed_cyclotomic_square_block_layout_metrics, fp12_cyclotomic_square_k,
  fp12_cyclotomic_square_layout_metrics, fp12_mul_by_unitary_inverse_layout_metrics, fp12_mul_k,
  fp12_mul_layout_metrics, fp12_nonresidue, fp12_square_k, fp12_square_layout_metrics,
  g1_add_k, g1_add_layout_metrics, g2_curve_coeff_b,
  g2_double_with_line_k, g2_double_with_line_layout_metrics, g2_mixed_add_with_line_k,
  g2_mixed_add_with_line_layout_metrics, g2_neg_k, g2_neg_layout_metrics, g2_on_curve_k,
  g2_on_curve_layout_metrics, g2_proj_add_k, g2_proj_add_layout_metrics, g2_proj_double_k,
  g2_proj_double_layout_metrics, g2_proj_from_affine_k, g2_proj_from_affine_layout_metrics,
  miller_accumulator_mul_by_line_k, miller_accumulator_mul_by_line_layout_metrics,
  miller_accumulator_mul_by_line_sparse_k, miller_accumulator_mul_by_line_sparse_layout_metrics,
  miller_accumulator_square_k, miller_accumulator_square_layout_metrics, miller_loop,
  miller_loop_k, miller_loop_layout_metrics, multi_miller_loop, pairing_check, pairing_check_k,
  pairing_check_layout_metrics, retained_bn254_exp_by_x_chain_candidate,
  search_bn254_exp_by_x_candidates, search_bn254_exp_by_x_candidates_with_windows,
};
use ff::{Field, FromUniformBytes};
#[cfg(feature = "test")]
pub use groth16::fixtures::{raw as groth16_fixture_raw, typed as groth16_fixture_typed};
pub use groth16::profiling::{
  PAIRING_TERM_PROFILE_COUNTS, PUBLIC_INPUT_PROFILE_COUNTS,
  groth16_fixture_ic_accumulator_layout_metrics, groth16_fixture_verifier_layout_metrics,
  groth16_fixture_ic_accumulator_layout_metrics_v1, groth16_fixture_verifier_layout_metrics_v1,
  groth16_pairing_block_final_exponentiation_easy_part_layout_metrics,
  groth16_pairing_block_final_exponentiation_easy_part_layout_metrics_v1,
  groth16_pairing_block_final_exponentiation_hard_part_layout_metrics,
  groth16_pairing_block_final_exponentiation_hard_part_layout_metrics_v1,
  groth16_pairing_block_final_exponentiation_layout_metrics,
  groth16_pairing_block_final_exponentiation_layout_metrics_v1,
  groth16_pairing_block_miller_loop_layout_metrics,
  groth16_pairing_block_miller_loop_layout_metrics_v1,
  groth16_pairing_block_pairing_check_groth16_style_layout_metrics,
  groth16_pairing_block_pairing_check_groth16_style_layout_metrics_v1,
  groth16_pairing_block_pairing_check_layout_metrics, groth16_pairing_term_count_layout_metrics,
  groth16_pairing_block_pairing_check_layout_metrics_v1, groth16_public_input_count_layout_metrics,
  outer_wrapper_fixture_layout_metrics, outer_wrapper_fixture_layout_metrics_for_host,
};
#[cfg(feature = "test")]
pub use groth16::reference::host_verify;
pub use groth16::{
  Groth16Bn254G1Point, Groth16Bn254Proof, Groth16Bn254VerifierCircuit, Groth16Bn254VerifyingKey,
  Groth16IcAccumulatorCircuit, Groth16VerifierError, groth16_accumulate_ic, groth16_verify,
};
pub use metrics::{CostEstimate, LayoutMetrics};
use midnight_circuits::midnight_proofs::{dev::cost_model::circuit_model, plonk::Circuit};
pub use outer::{
  CircuitBuildStatus, HostedOuterWrapperCircuit, HostedOuterWrapperCircuitBls12,
  HostedOuterWrapperCircuitBn254, InnerVerifierFlavor, MidnightBls12_381HostConfigShell,
  MidnightBls12_381HostLane, MidnightBn254HostConfig, MidnightBn254HostLane,
  OuterArtifactSerializationFlavor, OuterCanonicalR1csLoweringError,
  OuterCanonicalR1csLoweringReport, OuterCanonicalR1csSliceKind, OuterCanonicalR1csSliceReport,
  OuterCanonicalR1csSliceStatus, OuterGroth16IcAccumulatorSlice,
  OuterGroth16PairingProductCheckSlice, OuterHostConfig, OuterHostField, OuterHostFlavor,
  OuterHostLane, OuterStatementExposureR1cs, OuterStatementInput, OuterStatementSemantics,
  OuterVerifierResultAssertionSlice, OuterWrapperCircuit, OuterWrapperCircuitInput,
  OuterWrapperFlavorProfile, OuterWrapperHostConfig, OuterWrapperHostConfigBls12,
  OuterWrapperHostConfigBn254, build_outer_groth16_ic_accumulator_slice,
  build_outer_groth16_pairing_product_check_slice, build_outer_statement_exposure_r1cs,
  build_outer_verifier_result_assertion_slice, build_outer_wrapper_canonical_r1cs,
  build_outer_wrapper_circuit, inspect_outer_wrapper_canonical_r1cs, lift_outer_input_to_host,
  lift_outer_inputs_to_host,
};
pub use planning::{
  CircuitPlanningView, PRIMITIVE_COUNT, PrimitiveCostEntry, PrimitiveCostLayer, PrimitiveCostTable,
  PrimitiveDefinition, primitive_definitions,
};
pub use r1cs::{
  ArkworksPreparedVerifyingKey, ArkworksProof, ArkworksProvingKey, ArkworksR1csCircuit,
  ArkworksVerifyingKey, CanonicalCellUnionFind, CanonicalClassId, CanonicalR1csBuilder,
  EqualityEdge, Halo2CellAssignmentMap, Halo2CellLinearCombination, Halo2CellRef, Halo2CellTerm,
  Halo2Phase1R1csLowering, Halo2PublicInputRef, Halo2R1csMetadata, LinearCombination, LinearTerm,
  R1CS_IDENTITY_DOMAIN_SEPARATOR, R1csAssignment, R1csBuildError, R1csCircuit, R1csConstraint,
  R1csIdentityHash, VariableId, ZkInterfaceConstraint, ZkInterfaceLinearCombination,
  ZkInterfaceR1csExport, ZkInterfaceTerm, ZkInterfaceWitnessAssignment, ZkInterfaceWitnessExport,
  arkworks_create_random_proof, arkworks_generate_random_parameters, arkworks_verify_proof,
  export_witness, ordered_public_inputs, to_ark_lc,
};

/// Measures layout metrics for one native-field circuit using the shared cost model.
#[must_use]
pub fn measure_native_circuit_layout(circuit: &impl Circuit<NativeField>) -> LayoutMetrics {
  bn254::measure_layout(circuit)
}

/// Measures layout metrics for one arbitrary host-field circuit using the shared
/// Halo2/Midnight cost model.
#[must_use]
pub fn measure_host_circuit_layout<FHost>(circuit: &impl Circuit<FHost>) -> LayoutMetrics
where
  FHost: Ord + Field + FromUniformBytes<64>,
{
  LayoutMetrics::from(circuit_model::<FHost, 48, 32>(circuit))
}
