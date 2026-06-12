use ark_ff::Field as ArkField;
use ff::Field;
use midnight_proofs::dev::MockProver;
use wrapper_core::WrapperError;

use super::{
  CircuitBuildStatus, OuterCanonicalR1csLoweringError, OuterCanonicalR1csLoweringReport,
  OuterCanonicalR1csSliceKind, OuterCanonicalR1csSliceStatus, OuterGroth16IcAccumulatorSlice,
  OuterGroth16PairingProductCheckSlice, OuterHostFlavor, OuterStatementExposureR1cs,
  OuterStatementInput, OuterStatementSemantics, OuterVerifierResultAssertionSlice,
  OuterVerificationKeyCommitmentValue, OuterWrapperCircuit, OuterWrapperCircuitInput,
  build_outer_groth16_ic_accumulator_slice, build_outer_groth16_pairing_product_check_slice,
  build_outer_statement_exposure_r1cs, build_outer_verifier_result_assertion_slice,
  build_outer_wrapper_canonical_r1cs, build_outer_wrapper_circuit,
  inspect_outer_wrapper_canonical_r1cs,
};
use crate::{
  ForeignField, NativeField,
  groth16::fixtures::typed::{
    proof as fixture_proof, public_inputs as fixture_public_inputs, verifying_key as fixture_vk,
  },
};

fn canonical_input() -> OuterWrapperCircuitInput {
  OuterWrapperCircuitInput::explicit(
    fixture_proof(),
    fixture_vk(),
    fixture_public_inputs(),
    vec!["public_input_0".to_owned()],
  )
}

#[test]
fn outer_wrapper_circuit_input_accepts_canonical_fixture() {
  canonical_input()
    .validate_for_outer_host(OuterHostFlavor::MidnightBn254)
    .expect("canonical outer wrapper input should satisfy the frozen contract");
}

#[test]
fn outer_wrapper_circuit_reports_integrated_build_status() {
  let circuit = build_outer_wrapper_circuit(canonical_input());

  assert_eq!(circuit.build_status(), CircuitBuildStatus::VerifierIntegrated);
}

#[test]
fn outer_wrapper_circuit_can_materialize_hosted_wrapper() {
  let circuit = build_outer_wrapper_circuit(canonical_input());
  let hosted = circuit.hosted();

  assert_eq!(hosted.build_status(), CircuitBuildStatus::VerifierIntegrated);
  hosted.assert_ready_for_synthesis().expect("hosted wrapper should preserve semantic readiness");
}

#[test]
fn outer_wrapper_circuit_defaults_to_current_bn254_host_flavor() {
  let circuit = build_outer_wrapper_circuit(canonical_input());

  assert_eq!(circuit.flavors.outer_host, OuterHostFlavor::MidnightBn254);
}

#[test]
fn outer_wrapper_circuit_is_ready_for_synthesis_with_canonical_fixture() {
  let circuit = build_outer_wrapper_circuit(canonical_input());

  circuit
    .assert_ready_for_synthesis()
    .expect("canonical outer wrapper circuit should be ready for synthesis");
}

#[test]
fn outer_wrapper_circuit_rejects_public_input_length_mismatch() {
  let input = OuterWrapperCircuitInput::new(
    fixture_proof(),
    fixture_vk(),
    Vec::new(),
    OuterStatementInput::new(
      OuterStatementSemantics::MirrorInnerPublicInputsAndVerificationKeyCommitment,
      Vec::new(),
      Vec::new(),
      super::OuterVerificationKeyCommitment::new(
        "vk_commitment",
        OuterVerificationKeyCommitmentValue::Bn254(crate::ForeignField::ZERO),
      ),
    ),
  );
  let circuit = OuterWrapperCircuit::from_input(input);

  assert!(matches!(
    circuit.assert_ready_for_synthesis(),
    Err(WrapperError::InvalidInput { context: "outer wrapper circuit input", .. })
  ));
}

#[test]
fn outer_wrapper_circuit_rejects_inconsistent_ic_length() {
  let mut vk = fixture_vk();
  let _ = vk.ic.pop();
  let circuit = OuterWrapperCircuit::from_input(OuterWrapperCircuitInput::explicit(
    fixture_proof(),
    vk,
    fixture_public_inputs(),
    vec!["public_input_0".to_owned()],
  ));

  assert!(matches!(
    circuit.assert_ready_for_synthesis(),
    Err(WrapperError::InvalidInput { context: "outer wrapper circuit input", .. })
  ));
}

#[test]
fn outer_wrapper_circuit_rejects_non_mirrored_statement() {
  let input = OuterWrapperCircuitInput::new(
    fixture_proof(),
    fixture_vk(),
    fixture_public_inputs(),
    OuterStatementInput::new(
      OuterStatementSemantics::MirrorInnerPublicInputsAndVerificationKeyCommitment,
      vec!["public_input_0".to_owned()],
      vec![NativeField::from(999_u64)],
      super::OuterVerificationKeyCommitment::new(
        "vk_commitment",
        OuterVerificationKeyCommitmentValue::Bn254(crate::groth16_vk_commitment(&fixture_vk())),
      ),
    ),
  );
  let circuit = OuterWrapperCircuit::from_input(input);

  assert!(matches!(
    circuit.assert_ready_for_synthesis(),
    Err(WrapperError::InvalidInput { context: "outer statement", .. })
  ));
}

#[test]
fn outer_wrapper_circuit_rejects_mutated_public_vk_commitment() {
  let mut input = canonical_input();
  input.outer_statement = OuterStatementInput::new(
    input.outer_statement.semantics,
    input.outer_statement.mirrored_field_names.clone(),
    input.outer_statement.mirrored_public_inputs.clone(),
    super::OuterVerificationKeyCommitment::new(
      input.outer_statement.vk_commitment.field_name.clone(),
      OuterVerificationKeyCommitmentValue::Bn254(crate::groth16_vk_commitment(&fixture_vk()) + ForeignField::ONE),
    ),
  );
  let circuit = OuterWrapperCircuit::from_input(input);

  assert!(matches!(
    circuit.assert_ready_for_synthesis(),
    Err(WrapperError::InvalidInput { context: "outer statement", .. })
  ));
}

#[test]
fn outer_wrapper_circuit_rejects_mutated_witness_side_vk_with_unchanged_public_commitment() {
  let input = canonical_input();
  let original_commitment = input.outer_statement.vk_commitment.clone();
  let mut mutated_vk = input.inner_verification_key.clone();

  mutated_vk.alpha_g1 = match mutated_vk.alpha_g1 {
    crate::Groth16Bn254G1Point::Identity => {
      panic!("canonical fixture should not use the G1 identity in alpha_g1")
    }
    crate::Groth16Bn254G1Point::Affine { x, y } => {
      crate::Groth16Bn254G1Point::affine(x + ForeignField::ONE, y)
    }
  };

  let circuit = OuterWrapperCircuit::from_input(OuterWrapperCircuitInput::new(
    input.inner_proof,
    mutated_vk,
    input.inner_public_inputs,
    OuterStatementInput::new(
      OuterStatementSemantics::MirrorInnerPublicInputsAndVerificationKeyCommitment,
      input.outer_statement.mirrored_field_names,
      input.outer_statement.mirrored_public_inputs,
      original_commitment,
    ),
  ));

  assert!(matches!(
    circuit.assert_ready_for_synthesis(),
    Err(WrapperError::InvalidInput { context: "outer statement", .. })
  ));
}

#[test]
fn outer_wrapper_circuit_accepts_bls12_host_flavor_boundary() {
  let circuit = OuterWrapperCircuit::from_input_for_host(
    OuterWrapperCircuitInput::explicit_for_outer_host(
      fixture_proof(),
      fixture_vk(),
      fixture_public_inputs(),
      vec!["public_input_0".to_owned()],
      OuterHostFlavor::MidnightBls12_381,
    ),
    OuterHostFlavor::MidnightBls12_381,
  );

  assert!(circuit.assert_ready_for_synthesis().is_ok());
}

#[test]
fn outer_statement_exposure_slice_can_lower_to_canonical_r1cs() {
  let OuterStatementExposureR1cs { metadata, circuit } =
    build_outer_statement_exposure_r1cs(&canonical_input())
      .expect("canonical outer statement exposure should lower");

  let expected_public_inputs = canonical_input().outer_statement.public_inputs.len();
  assert_eq!(metadata.public_inputs.len(), expected_public_inputs);
  assert_eq!(metadata.equality_edges.len(), expected_public_inputs);
  assert_eq!(metadata.public_inputs[0].cell, crate::Halo2CellRef::Instance { column: 0, row: 0 });
  assert_eq!(circuit.public_inputs.len(), expected_public_inputs);
  assert_eq!(circuit.witnesses.len(), 0);
  assert!(circuit.constraints.is_empty());
}

#[test]
fn full_outer_wrapper_canonical_r1cs_lowering_is_still_rejected_for_verifier_body() {
  assert!(matches!(
    build_outer_wrapper_canonical_r1cs(&canonical_input()),
    Err(OuterCanonicalR1csLoweringError::UnsupportedVerifierBodyLowering { pending_slices })
      if pending_slices == vec![OuterCanonicalR1csSliceKind::Groth16PairingProductCheck]
  ));
}

#[test]
fn outer_wrapper_canonical_r1cs_report_tracks_completed_and_pending_slices() {
  let OuterCanonicalR1csLoweringReport {
    statement_exposure,
    ic_accumulator,
    pairing_product_check,
    verifier_result_assertion,
    slices,
  } = inspect_outer_wrapper_canonical_r1cs(&canonical_input())
    .expect("canonical outer wrapper input should produce a lowering report");

  assert_eq!(
    statement_exposure.circuit.public_inputs.len(),
    canonical_input().outer_statement.public_inputs.len()
  );
  assert_eq!(ic_accumulator.public_inputs, fixture_public_inputs());
  assert_eq!(ic_accumulator.ic_points, fixture_vk().ic);
  assert_eq!(ic_accumulator.public_input_variables.len(), 1);
  assert_eq!(ic_accumulator.scheduled_scalar_variables.len(), 1);
  assert_eq!(ic_accumulator.circuit.public_inputs.len(), 1);
  assert_eq!(ic_accumulator.circuit.witnesses.len(), 1);
  assert_eq!(ic_accumulator.circuit.constraints.len(), 1);
  assert!(pairing_product_check.expected_is_identity);
  assert!(verifier_result_assertion.expected_result);
  assert_eq!(verifier_result_assertion.circuit.public_inputs.len(), 0);
  assert_eq!(verifier_result_assertion.circuit.witnesses.len(), 1);
  assert_eq!(verifier_result_assertion.circuit.constraints.len(), 1);
  assert_eq!(slices.len(), 4);
  assert_eq!(slices[0].kind, OuterCanonicalR1csSliceKind::OuterStatementExposure);
  assert_eq!(slices[0].status, OuterCanonicalR1csSliceStatus::Lowered);
  assert_eq!(slices[1].kind, OuterCanonicalR1csSliceKind::Groth16IcAccumulator);
  assert_eq!(slices[1].status, OuterCanonicalR1csSliceStatus::Lowered);
  assert_eq!(slices[2].kind, OuterCanonicalR1csSliceKind::Groth16PairingProductCheck);
  assert!(matches!(slices[2].status, OuterCanonicalR1csSliceStatus::Prepared { .. }));
  assert_eq!(slices[3].kind, OuterCanonicalR1csSliceKind::VerifierResultAssertion);
  assert_eq!(slices[3].status, OuterCanonicalR1csSliceStatus::Lowered);
}

#[test]
fn outer_groth16_ic_accumulator_slice_extracts_deterministic_inputs_and_reference() {
  let OuterGroth16IcAccumulatorSlice {
    public_inputs,
    ic_points,
    public_input_variables,
    scheduled_scalar_variables,
    expected_accumulator,
    circuit,
  } = build_outer_groth16_ic_accumulator_slice(&canonical_input())
    .expect("canonical IC accumulator slice should extract deterministically");

  assert_eq!(public_inputs, fixture_public_inputs());
  assert_eq!(ic_points, fixture_vk().ic);
  assert_eq!(
    expected_accumulator,
    crate::groth16::groth16_public_input_accumulator_constant(
      &fixture_vk(),
      &fixture_public_inputs()
    )
  );
  assert_eq!(public_input_variables.len(), 1);
  assert_eq!(scheduled_scalar_variables.len(), 1);
  assert_eq!(circuit.public_inputs.len(), 1);
  assert_eq!(circuit.witnesses.len(), 1);
  assert_eq!(circuit.constraints.len(), 1);
}

#[test]
fn outer_verifier_result_assertion_slice_extracts_fixed_expected_result() {
  let OuterVerifierResultAssertionSlice {
    expected_result,
    assertion_rule,
    result_variable: _,
    circuit,
  } = build_outer_verifier_result_assertion_slice(&canonical_input())
    .expect("verifier-result assertion slice should extract deterministically");

  assert!(expected_result);
  assert!(assertion_rule.contains("equals true"));
  assert_eq!(circuit.constraints.len(), 1);
}

#[test]
fn outer_groth16_pairing_product_check_slice_extracts_deterministic_reference() {
  let OuterGroth16PairingProductCheckSlice {
    proof,
    verification_key,
    public_inputs,
    expected_pairing_product,
    expected_is_identity,
  } = build_outer_groth16_pairing_product_check_slice(&canonical_input())
    .expect("pairing-product check slice should extract deterministically");

  assert_eq!(proof, fixture_proof());
  assert_eq!(verification_key, fixture_vk());
  assert_eq!(public_inputs, fixture_public_inputs());
  assert!(expected_is_identity);
  assert_eq!(expected_pairing_product, ark_bn254::Fq12::ONE);
}

#[test]
#[ignore = "slow pairing-core"]
fn slow_mockprover_valid_outer_wrapper_fixture_is_satisfied() {
  let input = canonical_input();
  let instances = input.outer_statement.public_inputs.clone();
  let circuit = OuterWrapperCircuit::from_input(input).into_hosted();
  let prover =
    MockProver::run(22, &circuit, vec![instances, vec![]]).expect("MockProver should build");

  assert_eq!(prover.verify(), Ok(()));
}

#[test]
#[ignore = "slow pairing-core"]
fn slow_mockprover_wrong_outer_statement_instance_is_rejected() {
  let input = canonical_input();
  let circuit = OuterWrapperCircuit::from_input(input).into_hosted();
  let mut wrong_instances = canonical_input().outer_statement.public_inputs.clone();
  wrong_instances[0] = NativeField::from(34_u64);
  let prover =
    MockProver::run(22, &circuit, vec![wrong_instances, vec![]]).expect("MockProver should build");

  assert!(prover.verify().is_err(), "wrong instance should fail public-input exposure");
}
