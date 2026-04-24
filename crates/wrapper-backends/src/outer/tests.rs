use wrapper_circuits::groth16_fixture_raw;
use wrapper_core::{ProducedOuterProofJson, ProofSystemKind};

use crate::parse_snarkjs_groth16_bn254_bundle;

use super::{
  MidnightDirectOuterBackend, MidnightDirectOuterBackendBls12Host,
  MidnightDirectOuterBackendBn254Host, OuterCircuitInputArtifacts, OuterProofBackend,
  OuterProofBackendError, PlannedHalo2OuterBackend, PlannedHalo2OuterBackendBn254Host,
  current_reference_outer_backend, current_reference_outer_backend_metadata,
  current_reference_outer_host,
};

fn real_fixture_package() -> wrapper_core::WrapperExecutionPackage {
  parse_snarkjs_groth16_bn254_bundle(
    "circom-multiplier2",
    groth16_fixture_raw::proof_json(),
    groth16_fixture_raw::public_inputs_json(),
    groth16_fixture_raw::verification_key_json(),
  )
  .expect("fixture bundle should parse")
  .build_halo2_outer_execution_package()
}

#[test]
fn planned_backend_prepares_halo2_outer_placeholder_bundle() {
  let backend = PlannedHalo2OuterBackend;
  let planned = backend
    .prepare(&real_fixture_package())
    .expect("planned backend should accept halo2-outer target");

  assert_eq!(backend.backend_id(), "planned-halo2-outer-backend");
  assert_eq!(planned.bundle_template.proof_system.kind, ProofSystemKind::Halo2Outer);
  assert_eq!(
    planned
      .bundle_template
      .verification_key
      .as_ref()
      .expect("planned bundle should materialize a verification-key skeleton")
      .curve,
    "bn254"
  );
}

#[test]
fn direct_backend_exposes_selected_stack_metadata() {
  let metadata = MidnightDirectOuterBackend.metadata();
  let capabilities = metadata.capabilities();
  let proof_serialization = metadata.proof_serialization();
  let vk_serialization = metadata.verification_key_serialization();

  assert_eq!(metadata.backend_id, "midnight-direct-halo2-outer-backend-bn254-host");
  assert_eq!(metadata.inner_verifier, wrapper_circuits::InnerVerifierFlavor::Groth16Bn254);
  assert_eq!(metadata.outer_host, wrapper_circuits::OuterHostFlavor::MidnightBn254);
  assert_eq!(
    metadata.serialization,
    wrapper_circuits::OuterArtifactSerializationFlavor::SerdeJsonHexEncodedProcessed
  );
  assert_eq!(metadata.protocol, "halo2-plonkish");
  assert_eq!(metadata.curve, "bn254");
  assert_eq!(metadata.pcs, "kzg");
  assert_eq!(metadata.transcript, "blake2b");
  assert!(metadata.supports_setup);
  assert!(metadata.supports_prove);
  assert!(metadata.supports_verify);
  assert_eq!(capabilities.protocol, "halo2-plonkish");
  assert_eq!(capabilities.host_curve, "bn254");
  assert_eq!(capabilities.pcs, "kzg");
  assert_eq!(capabilities.transcript, "blake2b");
  assert!(capabilities.supports_setup);
  assert!(capabilities.supports_prove);
  assert!(capabilities.supports_verify);
  assert_eq!(proof_serialization.encoding, "hex");
  assert_eq!(vk_serialization.encoding, "hex");
  assert_eq!(proof_serialization.materialize("beef".to_owned()).backend, metadata.backend_id);
  assert_eq!(
    vk_serialization.materialize(19, 1, "cafe".to_owned(), "babe".to_owned()).backend,
    metadata.backend_id
  );
}

#[test]
fn current_bn254_lane_is_exposed_as_reference_backend_surface() {
  let backend = current_reference_outer_backend();
  let metadata = current_reference_outer_backend_metadata();
  let _: MidnightDirectOuterBackendBn254Host = backend;
  let _: PlannedHalo2OuterBackendBn254Host = PlannedHalo2OuterBackend;

  assert_eq!(current_reference_outer_host(), wrapper_circuits::OuterHostFlavor::MidnightBn254);
  assert_eq!(backend.backend_id(), MidnightDirectOuterBackend.backend_id());
  assert_eq!(metadata.outer_host, current_reference_outer_host());
  assert!(metadata.supports_setup);
  assert!(metadata.supports_prove);
  assert!(metadata.supports_verify);
}

#[test]
fn planned_bn254_lane_remains_compatibility_sibling_without_execution_capabilities() {
  let metadata = PlannedHalo2OuterBackend.metadata();
  let capabilities = PlannedHalo2OuterBackend.capabilities();

  assert_eq!(metadata.outer_host, current_reference_outer_host());
  assert_eq!(metadata.protocol, "halo2-plonkish");
  assert_eq!(metadata.curve, "bn254");
  assert_eq!(metadata.pcs, "kzg");
  assert_eq!(metadata.transcript, "blake2b");
  assert!(!capabilities.supports_setup);
  assert!(!capabilities.supports_prove);
  assert!(!capabilities.supports_verify);
}

#[test]
fn bls12_direct_lane_is_exposed_as_additive_placeholder_sibling() {
  let backend = MidnightDirectOuterBackendBls12Host;
  let metadata = backend.metadata();
  let planned = backend
    .prepare(&real_fixture_package())
    .expect("bls12 placeholder backend should still prepare honest artifact shapes");

  assert_eq!(metadata.outer_host, wrapper_circuits::OuterHostFlavor::MidnightBls12_381);
  assert_eq!(metadata.curve, "bls12-381");
  assert_eq!(metadata.pcs, "kzg");
  assert_eq!(metadata.transcript, "blake2b");
  assert!(!metadata.supports_setup);
  assert!(!metadata.supports_prove);
  assert!(!metadata.supports_verify);
  assert_eq!(planned.proof_shape.curve, "bls12-381");
  assert_eq!(planned.verification_key_shape.curve, "bls12-381");
  assert_eq!(planned.proof_shape.backend, metadata.backend_id);
  assert_eq!(planned.verification_key_shape.backend, metadata.backend_id);
}

#[test]
fn bls12_direct_lane_rejects_execution_until_host_lane_exists() {
  let backend = MidnightDirectOuterBackendBls12Host;
  let package = real_fixture_package();

  assert!(matches!(
    backend.setup(&package, OuterCircuitInputArtifacts::default()),
    Err(OuterProofBackendError::MissingDirectOuterCircuitBackend { .. })
  ));
  assert!(matches!(
    backend.prove(&package, OuterCircuitInputArtifacts::default()),
    Err(OuterProofBackendError::MissingDirectOuterCircuitBackend { .. })
  ));
}

#[test]
fn reference_bn254_lane_prepare_output_matches_reference_capabilities() {
  let backend = current_reference_outer_backend();
  let planned = backend
    .prepare(&real_fixture_package())
    .expect("reference backend should prepare a valid halo2-outer bundle");
  let capabilities = backend.capabilities();
  let metadata = backend.metadata();

  assert_eq!(planned.proof_shape.protocol, capabilities.protocol);
  assert_eq!(planned.proof_shape.curve, capabilities.host_curve);
  assert_eq!(planned.proof_shape.backend, metadata.backend_id);
  assert_eq!(planned.proof_shape.transcript, capabilities.transcript);
  assert_eq!(planned.proof_shape.payload_encoding, metadata.serialization.payload_encoding());
  assert_eq!(planned.verification_key_shape.protocol, capabilities.protocol);
  assert_eq!(planned.verification_key_shape.curve, capabilities.host_curve);
  assert_eq!(planned.verification_key_shape.backend, metadata.backend_id);
  assert_eq!(planned.verification_key_shape.pcs, capabilities.pcs);
  assert_eq!(
    planned.verification_key_shape.payload_encoding,
    metadata.serialization.payload_encoding()
  );
}

#[test]
fn direct_backend_builds_circuit_with_explicit_host_flavor_boundary() {
  let backend = MidnightDirectOuterBackend;
  let package = real_fixture_package();
  let circuit = backend
    .build_outer_circuit(
      &package,
      OuterCircuitInputArtifacts::new(
        Some(groth16_fixture_raw::proof_json()),
        Some(groth16_fixture_raw::verification_key_json()),
      ),
    )
    .expect("direct backend should build a valid canonical outer circuit");

  assert_eq!(circuit.flavors.inner_verifier, wrapper_circuits::InnerVerifierFlavor::Groth16Bn254);
  assert_eq!(circuit.flavors.outer_host, wrapper_circuits::OuterHostFlavor::MidnightBn254);
}

#[test]
#[ignore = "slow outer proving"]
fn direct_backend_can_plan_setup_and_produce_real_vk_artifact() {
  let backend = MidnightDirectOuterBackend;
  let package = real_fixture_package();
  let plan =
    backend.plan_setup(&package).expect("setup planning should succeed for a valid package");
  let vk = backend
    .setup(
      &package,
      OuterCircuitInputArtifacts::new(
        Some(groth16_fixture_raw::proof_json()),
        Some(groth16_fixture_raw::verification_key_json()),
      ),
    )
    .expect("setup should produce a real VK artifact");

  assert_eq!(plan.expected_public_input_count, 1);
  assert_eq!(plan.expected_pcs, "kzg");
  assert_eq!(vk.protocol, "halo2-plonkish");
  assert_eq!(vk.curve, "bn254");
  assert_eq!(vk.public_input_count, 1);
  assert!(!vk.verification_key.is_empty());
  assert!(!vk.verifier_params.is_empty());
}

#[test]
#[ignore = "slow outer proving"]
fn direct_backend_can_produce_real_proof_bundle() {
  let backend = MidnightDirectOuterBackend;
  let package = real_fixture_package();
  let bundle = backend
    .prove(
      &package,
      OuterCircuitInputArtifacts::new(
        Some(groth16_fixture_raw::proof_json()),
        Some(groth16_fixture_raw::verification_key_json()),
      ),
    )
    .expect("prove should produce a real proof bundle");

  assert_eq!(bundle.proof.protocol, "halo2-plonkish");
  assert_eq!(bundle.proof.curve, "bn254");
  assert_eq!(bundle.proof.backend, "midnight-direct-halo2-outer-backend");
  assert_eq!(bundle.proof.transcript, "blake2b");
  assert!(!bundle.proof.proof.is_empty());
  assert_eq!(bundle.verification_key.public_input_count, 1);
  assert_eq!(
    bundle.public_inputs,
    package
      .statement
      .public_inputs
      .entries
      .iter()
      .map(|entry| entry.value.clone())
      .collect::<Vec<_>>()
  );
}

#[test]
#[ignore = "slow outer proving"]
fn direct_backend_can_verify_real_proof_bundle() {
  let backend = MidnightDirectOuterBackend;
  let package = real_fixture_package();
  let bundle = backend
    .prove(
      &package,
      OuterCircuitInputArtifacts::new(
        Some(groth16_fixture_raw::proof_json()),
        Some(groth16_fixture_raw::verification_key_json()),
      ),
    )
    .expect("prove should produce a real proof bundle");

  assert!(
    backend
      .verify(
        &package,
        &bundle,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("verify should accept the produced proof bundle")
  );
}

#[test]
fn direct_backend_rejects_proof_with_wrong_curve() {
  let backend = MidnightDirectOuterBackend;
  let package = real_fixture_package();
  let proof = ProducedOuterProofJson {
    protocol: "halo2-plonkish".to_owned(),
    curve: "bls12-381".to_owned(),
    backend: "midnight-direct-halo2-outer-backend".to_owned(),
    transcript: "blake2b".to_owned(),
    encoding: "hex".to_owned(),
    proof: "beef".to_owned(),
  };

  assert!(matches!(
    backend.validate_produced_proof(&package, &proof),
    Err(OuterProofBackendError::ProofCurveMismatch { .. })
  ));
}
