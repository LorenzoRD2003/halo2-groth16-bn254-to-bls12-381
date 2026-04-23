//! Generic higher-level Groth16 BN254 artifact normalization helpers.

use wrapper_circuits::{Groth16Bn254Proof, Groth16Bn254VerifyingKey, NativeField};
use wrapper_core::{
  NamedPublicInput, NamedPublicInputs, ProofSystemDescriptor, ProofSystemKind, WrapperExecutionPackage,
  WrapperJob, WrapperStatement, WrapperWitnessInput,
};

use crate::loader::{ArtifactSetLoader, LoaderSummary};
use crate::snarkjs::{
  SnarkjsGroth16ParseError, parse_groth16_bn254_proof, parse_groth16_bn254_public_inputs,
  parse_groth16_bn254_public_inputs_with_names, parse_groth16_bn254_verifying_key,
};

/// Parsed Groth16 BN254 artifact set from a `snarkjs`-shaped source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Groth16Bn254ArtifactBundle {
  /// Logical identifier for the artifact set.
  pub identifier: String,
  /// Parsed Groth16 proof.
  pub proof: Groth16Bn254Proof,
  /// Parsed Groth16 verification key.
  pub verification_key: Groth16Bn254VerifyingKey,
  /// Parsed public-input vector in verifier order.
  pub public_inputs: Vec<NativeField>,
  /// Optional semantic names for the public-input vector.
  pub named_public_inputs: Option<NamedPublicInputs>,
}

impl Groth16Bn254ArtifactBundle {
  /// Returns the number of verifier public inputs.
  #[must_use]
  pub fn public_input_count(&self) -> usize {
    self.public_inputs.len()
  }

  /// Builds a generic wrapper job targeting the requested outer proof system.
  #[must_use]
  pub fn plan_wrapper_job(&self, target: ProofSystemDescriptor) -> WrapperJob {
    let mut notes = vec![
      "job is planned from a parsed Groth16 BN254 artifact bundle".to_owned(),
      "outer proof synthesis is not implemented in the current repository phase".to_owned(),
    ];

    if self.named_public_inputs.is_some() {
      notes.push("public inputs carry caller-supplied semantic names".to_owned());
    }

    WrapperJob::new(
      self.identifier.clone(),
      ProofSystemDescriptor {
        kind: ProofSystemKind::Groth16Bn254,
        source: "snarkjs-groth16-bn254-artifact-set-loader".to_owned(),
      },
      target,
      self.public_input_count(),
      self.named_public_inputs.clone(),
      notes,
    )
  }

  /// Builds a wrapper job for the current migration target experiment.
  #[must_use]
  pub fn plan_bls12_381_wrapper_job(&self) -> WrapperJob {
    self.plan_wrapper_job(ProofSystemDescriptor {
      kind: ProofSystemKind::Groth16Bls12_381,
      source: "planned-groth16-bls12-381-wrapper".to_owned(),
    })
  }

  fn named_public_inputs_or_indexed(&self) -> NamedPublicInputs {
    self.named_public_inputs.clone().unwrap_or_else(|| {
      NamedPublicInputs::new(
        self
          .public_inputs
          .iter()
          .enumerate()
          .map(|(index, value)| NamedPublicInput::new(format!("public_input_{index}"), format!("{value:?}")))
          .collect(),
      )
    })
  }

  /// Builds a serializable execution package for a future wrapper executor.
  #[must_use]
  pub fn build_execution_package(&self, job: WrapperJob) -> WrapperExecutionPackage {
    let named_public_inputs = self.named_public_inputs_or_indexed();

    WrapperExecutionPackage::new(
      job.clone(),
      WrapperStatement::new(named_public_inputs.clone()),
      WrapperWitnessInput::new(
        self.identifier.clone(),
        job.source.clone(),
        named_public_inputs,
        self.verification_key.ic.len(),
        true,
        true,
        vec![
          "inner Groth16 proof bytes are expected to travel alongside this package".to_owned(),
          "inner verification-key bytes are expected to travel alongside this package".to_owned(),
        ],
      ),
    )
  }

  /// Builds a serializable execution package for the current BLS12-381 target experiment.
  #[must_use]
  pub fn build_bls12_381_execution_package(&self) -> WrapperExecutionPackage {
    self.build_execution_package(self.plan_bls12_381_wrapper_job())
  }
}

/// Stateless artifact-set loader for `snarkjs` Groth16 BN254 artifacts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SnarkjsGroth16Bn254ArtifactSetLoader;

/// Parses a complete `snarkjs` Groth16 BN254 artifact set.
///
/// This is the generic higher-level entry point for callers that want one
/// normalized bundle instead of separately invoking proof/VK/public-input
/// parsers.
pub fn parse_snarkjs_groth16_bn254_bundle(
  identifier: impl Into<String>,
  proof_json: &[u8],
  public_json: &[u8],
  verification_key_json: &[u8],
) -> Result<Groth16Bn254ArtifactBundle, SnarkjsGroth16ParseError> {
  Ok(Groth16Bn254ArtifactBundle {
    identifier: identifier.into(),
    proof: parse_groth16_bn254_proof(proof_json)?,
    verification_key: parse_groth16_bn254_verifying_key(verification_key_json)?,
    public_inputs: parse_groth16_bn254_public_inputs(public_json)?,
    named_public_inputs: None,
  })
}

/// Parses a complete `snarkjs` Groth16 BN254 artifact set and assigns semantic
/// names to the public-input vector.
pub fn parse_snarkjs_groth16_bn254_bundle_with_names(
  identifier: impl Into<String>,
  proof_json: &[u8],
  public_json: &[u8],
  verification_key_json: &[u8],
  field_names: &[&str],
) -> Result<Groth16Bn254ArtifactBundle, SnarkjsGroth16ParseError> {
  Ok(Groth16Bn254ArtifactBundle {
    identifier: identifier.into(),
    proof: parse_groth16_bn254_proof(proof_json)?,
    verification_key: parse_groth16_bn254_verifying_key(verification_key_json)?,
    public_inputs: parse_groth16_bn254_public_inputs(public_json)?,
    named_public_inputs: Some(parse_groth16_bn254_public_inputs_with_names(
      public_json,
      field_names,
    )?),
  })
}

impl ArtifactSetLoader for SnarkjsGroth16Bn254ArtifactSetLoader {
  type ArtifactSet = Groth16Bn254ArtifactBundle;
  type Error = SnarkjsGroth16ParseError;

  fn summary(&self) -> LoaderSummary {
    LoaderSummary {
      name: "snarkjs-groth16-bn254-artifact-set-loader",
      proof_loading_available: true,
      vk_loading_available: true,
      artifact_set_loading_available: true,
    }
  }

  fn load_artifact_set(
    &self,
    identifier: &str,
    proof_json: &[u8],
    public_json: &[u8],
    verification_key_json: &[u8],
  ) -> Result<Self::ArtifactSet, Self::Error> {
    parse_snarkjs_groth16_bn254_bundle(identifier, proof_json, public_json, verification_key_json)
  }
}

#[cfg(test)]
mod tests {
  use wrapper_circuits::{groth16_fixture_raw, groth16_fixture_typed};
  use wrapper_core::{NamedPublicInput, NamedPublicInputs, ProofSystemKind};

  use super::{
    SnarkjsGroth16Bn254ArtifactSetLoader, parse_snarkjs_groth16_bn254_bundle,
    parse_snarkjs_groth16_bn254_bundle_with_names,
  };
  use crate::loader::ArtifactSetLoader;

  const SEMAPHORE_FIELD_ORDER: [&str; 4] =
    ["merkle_root", "nullifier", "message_hash", "scope_hash"];

  fn semaphore_proof_json() -> &'static [u8] {
    include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/proof.json")
  }

  fn semaphore_public_inputs_json() -> &'static [u8] {
    include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/public.json")
  }

  fn semaphore_verification_key_json() -> &'static [u8] {
    include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/verification_key.json")
  }

  #[test]
  fn parses_canonical_fixture_into_bundle() {
    let bundle = parse_snarkjs_groth16_bn254_bundle(
      "circom-multiplier2",
      groth16_fixture_raw::proof_json(),
      groth16_fixture_raw::public_inputs_json(),
      groth16_fixture_raw::verification_key_json(),
    )
    .expect("canonical fixture bundle should parse");

    assert_eq!(bundle.identifier, "circom-multiplier2");
    assert_eq!(bundle.proof, groth16_fixture_typed::proof());
    assert_eq!(bundle.verification_key, groth16_fixture_typed::verifying_key());
    assert_eq!(bundle.public_inputs, groth16_fixture_typed::public_inputs());
    assert_eq!(bundle.named_public_inputs, None);
  }

  #[test]
  fn parses_named_semaphore_bundle() {
    let bundle = parse_snarkjs_groth16_bn254_bundle_with_names(
      "semaphore-depth-10",
      semaphore_proof_json(),
      semaphore_public_inputs_json(),
      semaphore_verification_key_json(),
      &SEMAPHORE_FIELD_ORDER,
    )
    .expect("named Semaphore bundle should parse");

    assert_eq!(bundle.identifier, "semaphore-depth-10");
    assert_eq!(bundle.public_input_count(), 4);
    assert_eq!(
      bundle.named_public_inputs,
      Some(NamedPublicInputs::new(vec![
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
      ]))
    );
  }

  #[test]
  fn artifact_set_loader_loads_canonical_fixture_bundle() {
    let loader = SnarkjsGroth16Bn254ArtifactSetLoader;
    let bundle = loader
      .load_artifact_set(
        "circom-multiplier2",
        groth16_fixture_raw::proof_json(),
        groth16_fixture_raw::public_inputs_json(),
        groth16_fixture_raw::verification_key_json(),
      )
      .expect("artifact-set loader should parse canonical fixture");

    assert_eq!(bundle.identifier, "circom-multiplier2");
    assert_eq!(bundle.proof, groth16_fixture_typed::proof());
    assert_eq!(bundle.verification_key, groth16_fixture_typed::verifying_key());
    assert_eq!(bundle.public_inputs, groth16_fixture_typed::public_inputs());
    assert_eq!(loader.summary().artifact_set_loading_available, true);
  }

  #[test]
  fn semaphore_bundle_can_plan_bls12_381_wrapper_job() {
    let bundle = parse_snarkjs_groth16_bn254_bundle_with_names(
      "semaphore-depth-10",
      semaphore_proof_json(),
      semaphore_public_inputs_json(),
      semaphore_verification_key_json(),
      &SEMAPHORE_FIELD_ORDER,
    )
    .expect("named Semaphore bundle should parse");
    let job = bundle.plan_bls12_381_wrapper_job();

    assert_eq!(job.identifier, "semaphore-depth-10");
    assert_eq!(job.source.kind, ProofSystemKind::Groth16Bn254);
    assert_eq!(job.target.kind, ProofSystemKind::Groth16Bls12_381);
    assert_eq!(job.public_input_count, 4);
    assert!(job.named_public_inputs.is_some());
  }

  #[test]
  fn semaphore_bundle_can_build_execution_package() {
    let bundle = parse_snarkjs_groth16_bn254_bundle_with_names(
      "semaphore-depth-10",
      semaphore_proof_json(),
      semaphore_public_inputs_json(),
      semaphore_verification_key_json(),
      &SEMAPHORE_FIELD_ORDER,
    )
    .expect("named Semaphore bundle should parse");
    let package = bundle.build_bls12_381_execution_package();

    assert_eq!(package.job.target.kind, ProofSystemKind::Groth16Bls12_381);
    assert_eq!(package.statement.public_inputs.field_order(), SEMAPHORE_FIELD_ORDER);
    assert_eq!(package.witness.verifier_public_inputs.field_order(), SEMAPHORE_FIELD_ORDER);
    assert!(package.witness.requires_inner_proof);
    assert!(package.witness.requires_verification_key);
  }

  #[test]
  fn semaphore_execution_package_passes_stub_executor() {
    let bundle = parse_snarkjs_groth16_bn254_bundle_with_names(
      "semaphore-depth-10",
      semaphore_proof_json(),
      semaphore_public_inputs_json(),
      semaphore_verification_key_json(),
      &SEMAPHORE_FIELD_ORDER,
    )
    .expect("named Semaphore bundle should parse");
    let result = bundle.build_bls12_381_execution_package().execute_stub();

    assert!(result.preflight_ok);
  }
}
