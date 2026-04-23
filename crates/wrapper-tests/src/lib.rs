//! Test harness crate for workspace-level fixtures and integration helpers.
#![allow(clippy::multiple_crate_versions)]

use wrapper_backends as _;
use wrapper_circuits as _;
use wrapper_core as _;

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use midnight_proofs as _;

/// Returns the example config bundled for integration tests.
#[must_use]
pub fn example_config() -> &'static str {
  include_str!("../fixtures/example-config.toml")
}

#[cfg(test)]
mod tests {
  use wrapper_backends::{
    ArtifactSetLoader, BackendRegistry, Groth16Bn254ArtifactBundle,
    SnarkjsGroth16Bn254ArtifactSetLoader,
    parse_snarkjs_groth16_bn254_bundle_with_names,
  };
  use wrapper_core::{NamedPublicInput, NamedPublicInputs, ProjectConfig};
  use wrapper_circuits::{
    CircuitPlanningView, NativeField, groth16_fixture_raw, groth16_fixture_typed, host_verify,
  };

  use super::example_config;

  const SEMAPHORE_PUBLIC_INPUT_NAMES: [&str; 4] =
    ["merkle_root", "nullifier", "message_hash", "scope_hash"];

  #[test]
  fn example_config_parses() {
    let config = ProjectConfig::from_toml_str(example_config()).expect("config should parse");
    let layout = CircuitPlanningView::from_config(config).describe();

    assert_eq!(layout.name, "wrapper-scaffold");
  }

  #[test]
  fn backend_registry_contains_placeholders() {
    let registry = BackendRegistry::scaffold();

    assert_eq!(registry.entries().len(), 2);
  }

  fn load_groth16_fixture() -> Groth16Bn254ArtifactBundle {
    let loader = SnarkjsGroth16Bn254ArtifactSetLoader;

    loader
      .load_artifact_set(
        "circom-multiplier2",
        groth16_fixture_raw::proof_json(),
        groth16_fixture_raw::public_inputs_json(),
        groth16_fixture_raw::verification_key_json(),
      )
      .expect("fixture bundle should parse")
  }

  fn load_semaphore_fixture() -> Groth16Bn254ArtifactBundle {
    let loader = SnarkjsGroth16Bn254ArtifactSetLoader;

    loader
      .load_artifact_set(
        "semaphore-depth-10",
        include_bytes!("../fixtures/groth16/semaphore/proof.json"),
        include_bytes!("../fixtures/groth16/semaphore/public.json"),
        include_bytes!("../fixtures/groth16/semaphore/verification_key.json"),
      )
      .expect("Semaphore fixture bundle should parse")
  }

  fn load_named_semaphore_public_inputs() -> NamedPublicInputs {
    parse_snarkjs_groth16_bn254_bundle_with_names(
      "semaphore-depth-10",
      include_bytes!("../fixtures/groth16/semaphore/proof.json"),
      include_bytes!("../fixtures/groth16/semaphore/public.json"),
      include_bytes!("../fixtures/groth16/semaphore/verification_key.json"),
      &SEMAPHORE_PUBLIC_INPUT_NAMES,
    )
    .expect("named Semaphore bundle should parse")
    .named_public_inputs
    .expect("named Semaphore bundle should expose named public inputs")
  }

  fn assert_canonical_fixture_public_inputs(public_inputs: &[NativeField]) {
    assert_eq!(public_inputs, groth16_fixture_typed::public_inputs());
  }

  #[test]
  fn groth16_real_snarkjs_fixture_is_accepted_end_to_end() {
    let bundle = load_groth16_fixture();
    assert_canonical_fixture_public_inputs(&bundle.public_inputs);

    assert!(host_verify(
      &bundle.verification_key,
      &bundle.proof,
      &bundle.public_inputs,
    ));
  }

  #[test]
  fn groth16_mutated_public_input_is_rejected_end_to_end() {
    let bundle = load_groth16_fixture();
    let mut public_inputs = bundle.public_inputs.clone();
    public_inputs[0] = NativeField::from(34_u64);

    assert!(!host_verify(
      &bundle.verification_key,
      &bundle.proof,
      &public_inputs,
    ));
  }

  #[test]
  fn semaphore_snarkjs_fixture_is_accepted_end_to_end() {
    let bundle = load_semaphore_fixture();

    assert_eq!(bundle.public_inputs.len(), 4);
    assert_eq!(bundle.verification_key.ic.len(), 5);
    assert!(host_verify(
      &bundle.verification_key,
      &bundle.proof,
      &bundle.public_inputs,
    ));
  }

  #[test]
  fn semaphore_mutated_public_input_is_rejected_end_to_end() {
    let bundle = load_semaphore_fixture();
    let mut public_inputs = bundle.public_inputs.clone();
    public_inputs[1] += NativeField::from(1_u64);

    assert!(!host_verify(
      &bundle.verification_key,
      &bundle.proof,
      &public_inputs,
    ));
  }

  #[test]
  fn semaphore_fixture_public_inputs_can_be_named_at_fixture_layer() {
    let named = load_named_semaphore_public_inputs();

    assert_eq!(named.field_order(), SEMAPHORE_PUBLIC_INPUT_NAMES);
    assert_eq!(
      named,
      NamedPublicInputs::new(vec![
        NamedPublicInput::new(
          "merkle_root",
          "4990292586352433503726012711155167179034286198473030768981544541070532815155",
        ),
        NamedPublicInput::new(
          "nullifier",
          "17540473064543782218297133630279824063352907908315494138425986188962403570231",
        ),
        NamedPublicInput::new(
          "message_hash",
          "8665846418922331996225934941481656421248110469944536651334918563951783029",
        ),
        NamedPublicInput::new(
          "scope_hash",
          "170164770795872309789133717676167925425155944778337387941930839678899666300",
        ),
      ])
    );
  }
}
