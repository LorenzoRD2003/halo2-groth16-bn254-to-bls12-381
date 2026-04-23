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
  use wrapper_backends::BackendRegistry;
  use wrapper_backends::{
    parse_groth16_bn254_proof, parse_groth16_bn254_public_inputs, parse_groth16_bn254_verifying_key,
  };
  use wrapper_circuits::{
    CircuitPlanningView, Groth16Bn254Proof, Groth16Bn254VerifyingKey, NativeField,
    groth16_fixture_raw, groth16_fixture_typed, host_verify,
  };
  use wrapper_core::ProjectConfig;

  use super::example_config;

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

  fn load_groth16_fixture() -> (Groth16Bn254VerifyingKey, Groth16Bn254Proof, Vec<NativeField>) {
    let vk = parse_groth16_bn254_verifying_key(groth16_fixture_raw::verification_key_json())
      .expect("fixture vk should parse");
    let proof = parse_groth16_bn254_proof(groth16_fixture_raw::proof_json())
      .expect("fixture proof should parse");
    let public_inputs =
      parse_groth16_bn254_public_inputs(groth16_fixture_raw::public_inputs_json())
        .expect("fixture public inputs should parse");

    (vk, proof, public_inputs)
  }

  fn assert_canonical_fixture_public_inputs(public_inputs: &[NativeField]) {
    assert_eq!(public_inputs, groth16_fixture_typed::public_inputs());
  }

  #[test]
  fn groth16_real_snarkjs_fixture_is_accepted_end_to_end() {
    let (vk, proof, public_inputs) = load_groth16_fixture();
    assert_canonical_fixture_public_inputs(&public_inputs);

    assert!(host_verify(&vk, &proof, &public_inputs));
  }

  #[test]
  fn groth16_mutated_public_input_is_rejected_end_to_end() {
    let (vk, proof, mut public_inputs) = load_groth16_fixture();
    public_inputs[0] = NativeField::from(34_u64);

    assert!(!host_verify(&vk, &proof, &public_inputs));
  }
}
