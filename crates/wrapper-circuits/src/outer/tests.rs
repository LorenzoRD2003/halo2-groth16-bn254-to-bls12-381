use midnight_proofs::dev::MockProver;
use wrapper_core::WrapperError;

use super::{
  CircuitBuildStatus, OuterStatementInput, OuterStatementSemantics, OuterWrapperCircuit,
  OuterWrapperCircuitInput, build_outer_wrapper_circuit,
};
use crate::{
  NativeField,
  groth16::fixtures::typed::{
    proof as fixture_proof, public_inputs as fixture_public_inputs, verifying_key as fixture_vk,
  },
};

fn canonical_input() -> OuterWrapperCircuitInput {
  OuterWrapperCircuitInput::mirrored(
    fixture_proof(),
    fixture_vk(),
    fixture_public_inputs(),
    vec!["public_input_0".to_owned()],
  )
}

#[test]
fn outer_wrapper_circuit_input_accepts_canonical_fixture() {
  canonical_input()
    .validate()
    .expect("canonical outer wrapper input should satisfy the frozen contract");
}

#[test]
fn outer_wrapper_circuit_reports_integrated_build_status() {
  let circuit = build_outer_wrapper_circuit(canonical_input());

  assert_eq!(circuit.build_status(), CircuitBuildStatus::VerifierIntegrated);
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
      OuterStatementSemantics::MirrorInnerPublicInputs,
      Vec::new(),
      Vec::new(),
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
  let circuit = OuterWrapperCircuit::from_input(OuterWrapperCircuitInput::mirrored(
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
      OuterStatementSemantics::MirrorInnerPublicInputs,
      vec!["public_input_0".to_owned()],
      vec![NativeField::from(999_u64)],
    ),
  );
  let circuit = OuterWrapperCircuit::from_input(input);

  assert!(matches!(
    circuit.assert_ready_for_synthesis(),
    Err(WrapperError::InvalidInput { context: "outer statement", .. })
  ));
}

#[test]
#[ignore = "slow pairing-core"]
fn slow_mockprover_valid_outer_wrapper_fixture_is_satisfied() {
  let input = canonical_input();
  let instances = input.outer_statement.public_inputs.clone();
  let circuit = OuterWrapperCircuit::from_input(input);
  let prover =
    MockProver::run(22, &circuit, vec![instances, vec![]]).expect("MockProver should build");

  assert_eq!(prover.verify(), Ok(()));
}

#[test]
#[ignore = "slow pairing-core"]
fn slow_mockprover_wrong_outer_statement_instance_is_rejected() {
  let input = canonical_input();
  let circuit = OuterWrapperCircuit::from_input(input);
  let prover = MockProver::run(22, &circuit, vec![vec![NativeField::from(34_u64)], vec![]])
    .expect("MockProver should build");

  assert!(prover.verify().is_err(), "wrong instance should fail public-input exposure");
}
