//! Expected wrapper output shapes.

use serde::{Deserialize, Serialize};

use crate::{ProofSystemDescriptor, ProofSystemKind, WrapperStatement};

/// Stable canonical circuit identity carried alongside planned or produced artifacts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CanonicalCircuitIdentity {
  /// Identity scheme name.
  pub scheme: String,
  /// Stable identity value under that scheme.
  pub value: String,
}

impl CanonicalCircuitIdentity {
  /// Builds a canonical circuit identity record.
  #[must_use]
  pub fn new(scheme: impl Into<String>, value: impl Into<String>) -> Self {
    Self { scheme: scheme.into(), value: value.into() }
  }
}

/// Planned JSON payload for the future outer proof artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedOuterProofJson {
  /// Proof-system protocol label.
  pub protocol: String,
  /// Commitment-curve label.
  pub curve: String,
  /// Stable backend identifier.
  pub backend: String,
  /// Transcript family used by the backend.
  pub transcript: String,
  /// Payload encoding label.
  pub encoding: String,
  /// Placeholder serialized proof payload.
  pub proof: String,
}

impl PlannedOuterProofJson {
  /// Builds a placeholder outer-proof payload.
  #[must_use]
  pub fn placeholder(
    protocol: impl Into<String>,
    curve: impl Into<String>,
    backend: impl Into<String>,
    transcript: impl Into<String>,
    encoding: impl Into<String>,
  ) -> Self {
    Self {
      protocol: protocol.into(),
      curve: curve.into(),
      backend: backend.into(),
      transcript: transcript.into(),
      encoding: encoding.into(),
      proof: "<pending-real-proof>".to_owned(),
    }
  }
}

/// Planned JSON payload for the future outer verification-key artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedOuterVerificationKeyJson {
  /// Proof-system protocol label.
  pub protocol: String,
  /// Commitment-curve label.
  pub curve: String,
  /// Stable backend identifier.
  pub backend: String,
  /// Polynomial-commitment scheme label.
  pub pcs: String,
  /// Payload encoding label.
  pub encoding: String,
  /// Circuit size parameter when already known.
  pub circuit_k: Option<u32>,
  /// Ordered public-input count.
  pub public_input_count: usize,
  /// Placeholder serialized verification-key payload.
  pub verification_key: String,
  /// Placeholder serialized verifier-parameter payload.
  pub verifier_params: String,
}

impl PlannedOuterVerificationKeyJson {
  /// Builds a placeholder outer verification-key payload.
  #[must_use]
  pub fn placeholder(
    protocol: impl Into<String>,
    curve: impl Into<String>,
    backend: impl Into<String>,
    pcs: impl Into<String>,
    encoding: impl Into<String>,
    public_input_count: usize,
  ) -> Self {
    Self {
      protocol: protocol.into(),
      curve: curve.into(),
      backend: backend.into(),
      pcs: pcs.into(),
      encoding: encoding.into(),
      circuit_k: None,
      public_input_count,
      verification_key: "<pending-verification-key>".to_owned(),
      verifier_params: "<pending-verifier-params>".to_owned(),
    }
  }
}

/// Planned/materialized outer artifact bundle before real proving exists.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedOuterProofArtifactBundle {
  /// Output proof system represented by the bundle.
  pub proof_system: ProofSystemDescriptor,
  /// Canonical circuit identity tied to this bundle when available.
  pub canonical_circuit_identity: Option<CanonicalCircuitIdentity>,
  /// Logical proof artifact identifier.
  pub proof_artifact: String,
  /// Materialized proof payload when available.
  pub proof: Option<PlannedOuterProofJson>,
  /// Logical public-input artifact identifier.
  pub public_inputs_artifact: String,
  /// Planned public-input payload.
  pub public_inputs: Vec<String>,
  /// Logical verification-key artifact identifier.
  pub verification_key_artifact: String,
  /// Materialized verification-key payload when available.
  pub verification_key: Option<PlannedOuterVerificationKeyJson>,
  /// Notes about materialization state.
  pub notes: Vec<String>,
}

impl PlannedOuterProofArtifactBundle {
  /// Builds an outer artifact bundle from explicit payloads.
  #[must_use]
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    proof_system: ProofSystemDescriptor,
    canonical_circuit_identity: Option<CanonicalCircuitIdentity>,
    proof_artifact: impl Into<String>,
    proof: Option<PlannedOuterProofJson>,
    public_inputs_artifact: impl Into<String>,
    public_inputs: Vec<String>,
    verification_key_artifact: impl Into<String>,
    verification_key: Option<PlannedOuterVerificationKeyJson>,
    notes: Vec<String>,
  ) -> Self {
    Self {
      proof_system,
      canonical_circuit_identity,
      proof_artifact: proof_artifact.into(),
      proof,
      public_inputs_artifact: public_inputs_artifact.into(),
      public_inputs,
      verification_key_artifact: verification_key_artifact.into(),
      verification_key,
      notes,
    }
  }

  /// Builds a currently materializable outer artifact bundle from a wrapper statement.
  #[must_use]
  pub fn placeholder(
    identifier: impl Into<String>,
    protocol: impl Into<String>,
    curve: impl Into<String>,
    backend: impl Into<String>,
    pcs: impl Into<String>,
    encoding: impl Into<String>,
    transcript: impl Into<String>,
    statement: &WrapperStatement,
  ) -> Self {
    let identifier = identifier.into();
    let protocol = protocol.into();
    let curve = curve.into();
    let backend = backend.into();
    let pcs = pcs.into();
    let encoding = encoding.into();
    let transcript = transcript.into();

    Self {
      proof_system: ProofSystemDescriptor {
        kind: ProofSystemKind::Halo2Outer,
        source: backend.clone(),
      },
      canonical_circuit_identity: None,
      proof_artifact: format!("{identifier}-wrapper-proof.json"),
      proof: None,
      public_inputs_artifact: format!("{identifier}-wrapper-public.json"),
      public_inputs: statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| entry.value.clone())
        .collect(),
      verification_key_artifact: format!("{identifier}-wrapper-verification-key.json"),
      verification_key: Some(PlannedOuterVerificationKeyJson::placeholder(
        protocol.clone(),
        curve.clone(),
        backend.clone(),
        pcs,
        encoding.clone(),
        statement.public_inputs.entries.len(),
      )),
      notes: vec![
        format!(
          "planned {protocol}/{curve} outer bundle preserves the wrapper public inputs for backend {backend}"
        ),
        format!("proof payload remains absent until a real {backend} prover exists"),
        format!(
          "verification-key payload preserves the final serde-driven JSON contract with {encoding} payload encoding and transcript {transcript}"
        ),
      ],
    }
  }
}

/// Produced JSON payload for a real outer proof artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProducedOuterProofJson {
  /// Proof-system protocol label.
  pub protocol: String,
  /// Commitment-curve label.
  pub curve: String,
  /// Stable backend identifier.
  pub backend: String,
  /// Transcript family used by the backend.
  pub transcript: String,
  /// Payload encoding label.
  pub encoding: String,
  /// Serialized proof payload.
  pub proof: String,
}

/// Produced JSON payload for a real outer verification-key artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProducedOuterVerificationKeyJson {
  /// Proof-system protocol label.
  pub protocol: String,
  /// Commitment-curve label.
  pub curve: String,
  /// Stable backend identifier.
  pub backend: String,
  /// Polynomial-commitment scheme label.
  pub pcs: String,
  /// Payload encoding label.
  pub encoding: String,
  /// Real circuit size parameter used during setup.
  pub circuit_k: u32,
  /// Ordered public-input count.
  pub public_input_count: usize,
  /// Serialized verification-key payload.
  pub verification_key: String,
  /// Serialized verifier-parameter payload.
  pub verifier_params: String,
}

/// Strict produced outer artifact bundle emitted by a real backend.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProducedOuterProofArtifactBundle {
  /// Output proof system represented by the bundle.
  pub proof_system: ProofSystemDescriptor,
  /// Canonical circuit identity tied to this bundle when available.
  pub canonical_circuit_identity: Option<CanonicalCircuitIdentity>,
  /// Logical proof artifact identifier.
  pub proof_artifact: String,
  /// Produced proof payload.
  pub proof: ProducedOuterProofJson,
  /// Logical public-input artifact identifier.
  pub public_inputs_artifact: String,
  /// Produced public-input payload.
  pub public_inputs: Vec<String>,
  /// Logical verification-key artifact identifier.
  pub verification_key_artifact: String,
  /// Produced verification-key payload.
  pub verification_key: ProducedOuterVerificationKeyJson,
  /// Notes about production/setup state.
  pub notes: Vec<String>,
}

impl ProducedOuterProofArtifactBundle {
  /// Builds a produced outer artifact bundle from explicit payloads.
  #[must_use]
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    proof_system: ProofSystemDescriptor,
    canonical_circuit_identity: Option<CanonicalCircuitIdentity>,
    proof_artifact: impl Into<String>,
    proof: ProducedOuterProofJson,
    public_inputs_artifact: impl Into<String>,
    public_inputs: Vec<String>,
    verification_key_artifact: impl Into<String>,
    verification_key: ProducedOuterVerificationKeyJson,
    notes: Vec<String>,
  ) -> Self {
    Self {
      proof_system,
      canonical_circuit_identity,
      proof_artifact: proof_artifact.into(),
      proof,
      public_inputs_artifact: public_inputs_artifact.into(),
      public_inputs,
      verification_key_artifact: verification_key_artifact.into(),
      verification_key,
      notes,
    }
  }
}

/// Planned serialized shape for the outer proof artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExpectedProofArtifactShape {
  /// Serialization family expected from the future executor.
  pub format: String,
  /// Protocol label expected inside the proof artifact.
  pub protocol: String,
  /// Curve label expected inside the proof artifact.
  pub curve: String,
  /// Stable backend identifier expected inside the proof artifact.
  pub backend: String,
  /// Top-level JSON keys expected in the proof artifact.
  pub top_level_keys: Vec<String>,
  /// Transcript label expected inside the proof artifact.
  pub transcript: String,
  /// Expected key holding the payload encoding label.
  pub encoding_key: String,
  /// Expected key holding the serialized proof payload.
  pub proof_key: String,
  /// Expected serialized payload encoding.
  pub payload_encoding: String,
}

impl ExpectedProofArtifactShape {
  /// Builds the expected proof artifact shape.
  #[must_use]
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    format: impl Into<String>,
    protocol: impl Into<String>,
    curve: impl Into<String>,
    backend: impl Into<String>,
    top_level_keys: Vec<String>,
    transcript: impl Into<String>,
    encoding_key: impl Into<String>,
    proof_key: impl Into<String>,
    payload_encoding: impl Into<String>,
  ) -> Self {
    Self {
      format: format.into(),
      protocol: protocol.into(),
      curve: curve.into(),
      backend: backend.into(),
      top_level_keys,
      transcript: transcript.into(),
      encoding_key: encoding_key.into(),
      proof_key: proof_key.into(),
      payload_encoding: payload_encoding.into(),
    }
  }
}

/// Planned serialized shape for the outer public-input artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExpectedPublicInputsArtifactShape {
  /// Serialization family expected from the future executor.
  pub format: String,
  /// Top-level JSON/container shape expected from the artifact.
  pub container: String,
  /// Encoding used for each public-input element.
  pub element_encoding: String,
}

impl ExpectedPublicInputsArtifactShape {
  /// Builds the expected public-input artifact shape.
  #[must_use]
  pub fn new(
    format: impl Into<String>,
    container: impl Into<String>,
    element_encoding: impl Into<String>,
  ) -> Self {
    Self {
      format: format.into(),
      container: container.into(),
      element_encoding: element_encoding.into(),
    }
  }
}

/// Planned serialized shape for the outer verification-key artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExpectedVerificationKeyArtifactShape {
  /// Serialization family expected from the future executor.
  pub format: String,
  /// Protocol label expected inside the verification-key artifact.
  pub protocol: String,
  /// Curve label expected inside the verification-key artifact.
  pub curve: String,
  /// Stable backend identifier expected inside the verification-key artifact.
  pub backend: String,
  /// Top-level JSON keys expected in the verification-key artifact.
  pub top_level_keys: Vec<String>,
  /// Polynomial-commitment scheme label expected inside the verification-key artifact.
  pub pcs: String,
  /// Expected key holding the payload encoding label.
  pub encoding_key: String,
  /// Expected key holding the circuit size.
  pub circuit_size_key: String,
  /// Expected key holding the public-input count.
  pub public_input_count_key: String,
  /// Expected key holding the serialized verification key.
  pub verification_key_key: String,
  /// Expected key holding the serialized verifier params.
  pub verifier_params_key: String,
  /// Expected serialized payload encoding.
  pub payload_encoding: String,
}

impl ExpectedVerificationKeyArtifactShape {
  /// Builds the expected verification-key artifact shape.
  #[must_use]
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    format: impl Into<String>,
    protocol: impl Into<String>,
    curve: impl Into<String>,
    backend: impl Into<String>,
    top_level_keys: Vec<String>,
    pcs: impl Into<String>,
    encoding_key: impl Into<String>,
    circuit_size_key: impl Into<String>,
    public_input_count_key: impl Into<String>,
    verification_key_key: impl Into<String>,
    verifier_params_key: impl Into<String>,
    payload_encoding: impl Into<String>,
  ) -> Self {
    Self {
      format: format.into(),
      protocol: protocol.into(),
      curve: curve.into(),
      backend: backend.into(),
      top_level_keys,
      pcs: pcs.into(),
      encoding_key: encoding_key.into(),
      circuit_size_key: circuit_size_key.into(),
      public_input_count_key: public_input_count_key.into(),
      verification_key_key: verification_key_key.into(),
      verifier_params_key: verifier_params_key.into(),
      payload_encoding: payload_encoding.into(),
    }
  }
}

/// Expected wrapper artifacts once a real outer prover exists.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExpectedWrapperArtifacts {
  /// Output proof system expected from the wrapper executor.
  pub proof_system: ProofSystemDescriptor,
  /// Canonical circuit identity tied to these artifacts when available.
  pub canonical_circuit_identity: Option<CanonicalCircuitIdentity>,
  /// Logical proof artifact identifier.
  pub proof_artifact: String,
  /// Expected serialized proof artifact shape.
  pub proof_shape: ExpectedProofArtifactShape,
  /// Logical public-input artifact identifier.
  pub public_inputs_artifact: String,
  /// Expected serialized public-input artifact shape.
  pub public_inputs_shape: ExpectedPublicInputsArtifactShape,
  /// Logical verification-key artifact identifier.
  pub verification_key_artifact: String,
  /// Expected serialized verification-key artifact shape.
  pub verification_key_shape: ExpectedVerificationKeyArtifactShape,
  /// Public wrapper statement expected to match the emitted public-input artifact.
  pub statement: WrapperStatement,
  /// Currently materializable outer artifact bundle.
  pub bundle_template: PlannedOuterProofArtifactBundle,
  /// Planning notes about the expected outputs.
  pub notes: Vec<String>,
}

impl ExpectedWrapperArtifacts {
  /// Builds the expected output artifact description.
  #[must_use]
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    proof_system: ProofSystemDescriptor,
    canonical_circuit_identity: Option<CanonicalCircuitIdentity>,
    proof_artifact: impl Into<String>,
    proof_shape: ExpectedProofArtifactShape,
    public_inputs_artifact: impl Into<String>,
    public_inputs_shape: ExpectedPublicInputsArtifactShape,
    verification_key_artifact: impl Into<String>,
    verification_key_shape: ExpectedVerificationKeyArtifactShape,
    statement: WrapperStatement,
    bundle_template: PlannedOuterProofArtifactBundle,
    notes: Vec<String>,
  ) -> Self {
    Self {
      proof_system,
      canonical_circuit_identity,
      proof_artifact: proof_artifact.into(),
      proof_shape,
      public_inputs_artifact: public_inputs_artifact.into(),
      public_inputs_shape,
      verification_key_artifact: verification_key_artifact.into(),
      verification_key_shape,
      statement,
      bundle_template,
      notes,
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    CanonicalCircuitIdentity, ExpectedProofArtifactShape, ExpectedPublicInputsArtifactShape,
    ExpectedVerificationKeyArtifactShape, ExpectedWrapperArtifacts, NamedPublicInput,
    NamedPublicInputs, PlannedOuterProofArtifactBundle, ProducedOuterProofArtifactBundle,
    ProducedOuterProofJson, ProducedOuterVerificationKeyJson, ProofSystemDescriptor,
    ProofSystemKind, VerificationKeyCommitment, WrapperStatement,
  };

  #[test]
  fn expected_wrapper_artifacts_keep_statement_shape() {
    let statement = WrapperStatement::new(
      NamedPublicInputs::new(vec![
        NamedPublicInput::new("x", "1"),
        NamedPublicInput::new("y", "2"),
      ]),
      VerificationKeyCommitment::new(
        "vk_commitment",
        "7",
        NamedPublicInputs::new(vec![
          NamedPublicInput::new("vk_commitment_limb_0", "7"),
          NamedPublicInput::new("vk_commitment_limb_1", "0"),
        ]),
      ),
    );
    let artifacts = ExpectedWrapperArtifacts::new(
      ProofSystemDescriptor {
        kind: ProofSystemKind::Halo2Outer,
        source: "midnight-direct-halo2-outer-backend".to_owned(),
      },
      None,
      "proof.json",
      ExpectedProofArtifactShape::new(
        "json",
        "halo2-plonkish",
        "bn254",
        "midnight-direct-halo2-outer-backend",
        vec![
          "protocol".to_owned(),
          "curve".to_owned(),
          "backend".to_owned(),
          "transcript".to_owned(),
          "encoding".to_owned(),
          "proof".to_owned(),
        ],
        "blake2b",
        "encoding",
        "proof",
        "hex",
      ),
      "public.json",
      ExpectedPublicInputsArtifactShape::new("json", "array", "decimal-string"),
      "vk.json",
      ExpectedVerificationKeyArtifactShape::new(
        "json",
        "halo2-plonkish",
        "bn254",
        "midnight-direct-halo2-outer-backend",
        vec![
          "protocol".to_owned(),
          "curve".to_owned(),
          "backend".to_owned(),
          "pcs".to_owned(),
          "encoding".to_owned(),
          "circuit_k".to_owned(),
          "public_input_count".to_owned(),
          "verification_key".to_owned(),
          "verifier_params".to_owned(),
        ],
        "kzg",
        "encoding",
        "circuit_k",
        "public_input_count",
        "verification_key",
        "verifier_params",
        "hex",
      ),
      statement.clone(),
      PlannedOuterProofArtifactBundle::placeholder(
        "test-artifact",
        "halo2-plonkish",
        "bn254",
        "midnight-direct-halo2-outer-backend",
        "kzg",
        "hex",
        "blake2b",
        &statement,
      ),
      vec![],
    );

    assert_eq!(
      artifacts.statement.public_inputs.field_order(),
      vec!["x", "y", "vk_commitment_limb_0", "vk_commitment_limb_1"]
    );
    assert_eq!(artifacts.public_inputs_shape.container, "array");
    assert_eq!(artifacts.canonical_circuit_identity, None);
    assert_eq!(artifacts.verification_key_shape.protocol, "halo2-plonkish");
    assert_eq!(artifacts.proof_shape.proof_key, "proof");
    assert_eq!(artifacts.verification_key_shape.verification_key_key, "verification_key");
    assert_eq!(artifacts.bundle_template.public_inputs, vec!["1", "2", "7", "0"]);
    assert_eq!(artifacts.bundle_template.canonical_circuit_identity, None);
    assert!(artifacts.bundle_template.proof.is_none());
    assert_eq!(
      artifacts
        .bundle_template
        .verification_key
        .as_ref()
        .expect("bundle template should materialize a VK skeleton")
        .public_input_count,
      4
    );
    assert_eq!(artifacts.bundle_template.proof_system.kind, ProofSystemKind::Halo2Outer);
  }

  #[test]
  fn produced_outer_bundle_requires_real_proof_and_verification_key() {
    let bundle = ProducedOuterProofArtifactBundle::new(
      ProofSystemDescriptor {
        kind: ProofSystemKind::Halo2Outer,
        source: "midnight-direct-halo2-outer-backend".to_owned(),
      },
      Some(CanonicalCircuitIdentity::new("halo2-wrapper-input-hash", "deadbeef")),
      "proof.json",
      ProducedOuterProofJson {
        protocol: "halo2-plonkish".to_owned(),
        curve: "bn254".to_owned(),
        backend: "midnight-direct-halo2-outer-backend".to_owned(),
        transcript: "blake2b".to_owned(),
        encoding: "hex".to_owned(),
        proof: "abcd".to_owned(),
      },
      "public.json",
      vec!["1".to_owned(), "2".to_owned()],
      "vk.json",
      ProducedOuterVerificationKeyJson {
        protocol: "halo2-plonkish".to_owned(),
        curve: "bn254".to_owned(),
        backend: "midnight-direct-halo2-outer-backend".to_owned(),
        pcs: "kzg".to_owned(),
        encoding: "hex".to_owned(),
        circuit_k: 19,
        public_input_count: 2,
        verification_key: "beef".to_owned(),
        verifier_params: "cafe".to_owned(),
      },
      vec![],
    );

    assert_eq!(bundle.proof.protocol, "halo2-plonkish");
    assert_eq!(
      bundle.canonical_circuit_identity,
      Some(CanonicalCircuitIdentity::new("halo2-wrapper-input-hash", "deadbeef"))
    );
    assert_eq!(bundle.verification_key.circuit_k, 19);
    assert_eq!(bundle.verification_key.public_input_count, 2);
  }
}
