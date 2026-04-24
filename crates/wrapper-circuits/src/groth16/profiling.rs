//! Reproducible layout-measurement helpers for the current Groth16 BN254 slice.
//!
//! These helpers intentionally stay narrow and fixture-driven:
//! - they reuse the existing `measure_layout(...)` cost-model path
//! - they avoid host-time profiling or generalized verifier abstractions
//! - they provide deterministic scenario construction for optimization baselines

use halo2curves::group::Group;

use crate::LayoutMetrics;
use crate::bn254::{
  ForeignCurve, PairingCheckCircuit, final_exponentiation_easy_part_layout_metrics,
  final_exponentiation_hard_part_layout_metrics, final_exponentiation_layout_metrics,
  measure_layout, miller_loop_layout_metrics, pairing_check_layout_metrics,
};
use crate::measure_host_circuit_layout;
use crate::outer::{OuterHostFlavor, OuterWrapperCircuit, OuterWrapperCircuitInput};

use super::{
  Groth16Bn254G1Point, Groth16Bn254VerifierCircuit, Groth16Bn254VerifyingKey,
  Groth16IcAccumulatorCircuit, Groth16VariablePairingTermConstant, fixtures,
  groth16_g1_affine_coordinates, groth16_negate_g1, groth16_split_first_variable_and_prepare_rest,
  groth16_verifier_pairing_term_constants,
};
use crate::bn254::NativeField;

/// Deterministic pairing-term counts used by the profiling CLI.
pub const PAIRING_TERM_PROFILE_COUNTS: &[usize] = &[1, 2, 3, 4];

/// Deterministic public-input counts used by the profiling CLI.
pub const PUBLIC_INPUT_PROFILE_COUNTS: &[usize] = &[1, 2, 4, 8, 16];

type PairingTermConstant = Groth16VariablePairingTermConstant;

fn repeated_ic_vk(public_input_count: usize) -> Groth16Bn254VerifyingKey {
  let mut vk = fixtures::typed::verifying_key();
  let repeated_ic_point =
    *vk.ic.get(1).expect("canonical Groth16 fixture should expose one non-constant IC point");
  vk.ic = std::iter::once(Groth16Bn254G1Point::Identity)
    .chain(std::iter::repeat(repeated_ic_point).take(public_input_count))
    .collect();
  vk
}

fn repeated_public_inputs(public_input_count: usize) -> Vec<NativeField> {
  std::iter::repeat(NativeField::from(33_u64)).take(public_input_count).collect()
}

fn pairing_term_profile_terms(term_count: usize) -> Vec<PairingTermConstant> {
  assert!(term_count > 0, "profiling term count must be positive");

  let proof = fixtures::typed::proof();
  let g1 = groth16_g1_affine_coordinates(proof.a);
  let neg_g1 = groth16_g1_affine_coordinates(groth16_negate_g1(proof.a));
  let g2 = proof.b;

  (0..term_count).map(|index| if index % 2 == 0 { (g1, g2) } else { (neg_g1, g2) }).collect()
}

fn pairing_term_profile_expected(term_count: usize) -> bool {
  term_count % 2 == 0
}

/// Measures the canonical Groth16 verifier circuit on the committed snarkjs fixture.
#[must_use]
pub fn groth16_fixture_verifier_layout_metrics() -> LayoutMetrics {
  measure_layout(&Groth16Bn254VerifierCircuit::new(
    fixtures::typed::verifying_key(),
    fixtures::typed::proof(),
    fixtures::typed::public_inputs(),
    true,
  ))
}

/// Measures the canonical Groth16 verifier-side `vk_x` accumulator on the committed fixture.
#[must_use]
pub fn groth16_fixture_ic_accumulator_layout_metrics() -> LayoutMetrics {
  measure_layout(&Groth16IcAccumulatorCircuit::new(
    fixtures::typed::verifying_key(),
    fixtures::typed::public_inputs(),
    ForeignCurve::identity(),
  ))
}

/// Measures the canonical outer wrapper circuit on the committed Week 5 fixture.
#[must_use]
pub fn outer_wrapper_fixture_layout_metrics() -> LayoutMetrics {
  outer_wrapper_fixture_layout_metrics_for_host(OuterHostFlavor::MidnightBn254)
}

/// Measures the canonical outer wrapper circuit on the committed Week 5 fixture
/// for one explicit host lane.
#[must_use]
pub fn outer_wrapper_fixture_layout_metrics_for_host(outer_host: OuterHostFlavor) -> LayoutMetrics {
  let circuit = OuterWrapperCircuit::from_input_for_host(
    OuterWrapperCircuitInput::mirrored(
      fixtures::typed::proof(),
      fixtures::typed::verifying_key(),
      fixtures::typed::public_inputs(),
      vec!["public_input_0".to_owned()],
    ),
    outer_host,
  );

  match outer_host {
    OuterHostFlavor::MidnightBn254 => measure_layout(&circuit.hosted_bn254()),
    OuterHostFlavor::MidnightBls12_381 => measure_host_circuit_layout(&circuit.hosted_bls12()),
  }
}

/// Measures an isolated pairing-check circuit for a given number of pairing terms.
///
/// This is a deterministic term-count proxy for the Groth16 reduction. It keeps
/// the circuit narrow and stable while varying only the number of pairing terms.
#[must_use]
pub fn groth16_pairing_term_count_layout_metrics(term_count: usize) -> LayoutMetrics {
  let terms = pairing_term_profile_terms(term_count);
  let (variable_terms, prepared_terms) = groth16_split_first_variable_and_prepare_rest(&terms);
  measure_layout(&PairingCheckCircuit::new_with_prepared_constant_terms(
    &variable_terms,
    &prepared_terms,
    pairing_term_profile_expected(term_count),
  ))
}

/// Measures the Groth16 IC accumulator with a synthetic VK whose IC table grows
/// to match the requested number of public inputs.
///
/// The point/scalar values are deterministic and intentionally simple; this
/// isolates the current verifier-side accumulation shape without broadening the
/// repository into a generalized MSM API.
#[must_use]
pub fn groth16_public_input_count_layout_metrics(public_input_count: usize) -> LayoutMetrics {
  assert!(public_input_count > 0, "profiling public-input count must be positive");

  measure_layout(&Groth16IcAccumulatorCircuit::new(
    repeated_ic_vk(public_input_count),
    repeated_public_inputs(public_input_count),
    ForeignCurve::identity(),
  ))
}

/// Returns the current narrow Miller-loop layout metrics.
#[must_use]
pub fn groth16_pairing_block_miller_loop_layout_metrics() -> LayoutMetrics {
  miller_loop_layout_metrics()
}

/// Returns the current narrow final-exponentiation layout metrics.
#[must_use]
pub fn groth16_pairing_block_final_exponentiation_layout_metrics() -> LayoutMetrics {
  final_exponentiation_layout_metrics()
}

/// Returns the current narrow final-exponentiation easy-part layout metrics.
#[must_use]
pub fn groth16_pairing_block_final_exponentiation_easy_part_layout_metrics() -> LayoutMetrics {
  final_exponentiation_easy_part_layout_metrics()
}

/// Returns the current narrow final-exponentiation hard-part layout metrics.
#[must_use]
pub fn groth16_pairing_block_final_exponentiation_hard_part_layout_metrics() -> LayoutMetrics {
  final_exponentiation_hard_part_layout_metrics()
}

/// Returns the current narrow pairing-check primitive baseline.
#[must_use]
pub fn groth16_pairing_block_pairing_check_layout_metrics() -> LayoutMetrics {
  pairing_check_layout_metrics()
}

/// Returns a Groth16-shaped optimized pairing-check block:
/// one variable proof term plus three prepared constant verifier-key terms.
#[must_use]
pub fn groth16_pairing_block_pairing_check_groth16_style_layout_metrics() -> LayoutMetrics {
  let vk = fixtures::typed::verifying_key();
  let proof = fixtures::typed::proof();
  let public_inputs = fixtures::typed::public_inputs();
  let (variable_terms, prepared_terms) =
    groth16_verifier_pairing_term_constants(&vk, &proof, &public_inputs);

  measure_layout(&PairingCheckCircuit::new_with_prepared_constant_terms(
    &variable_terms,
    &prepared_terms,
    true,
  ))
}
