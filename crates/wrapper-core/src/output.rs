//! Expected wrapper output shapes.

use serde::{Deserialize, Serialize};

use crate::{ProofSystemDescriptor, WrapperStatement};

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

/// Placeholder G1 point payload using `snarkjs`-like projective JSON shape.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedGroth16G1PointJson {
  /// Projective x-coordinate.
  pub x: String,
  /// Projective y-coordinate.
  pub y: String,
  /// Projective z-coordinate.
  pub z: String,
}

impl PlannedGroth16G1PointJson {
  /// Builds a placeholder G1 point from a label stem.
  #[must_use]
  pub fn placeholder(label: impl Into<String>) -> Self {
    let label = label.into();
    Self { x: format!("<{}-x>", label), y: format!("<{}-y>", label), z: format!("<{}-z>", label) }
  }
}

/// Placeholder G2 point payload using `snarkjs`-like projective JSON shape.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedGroth16G2PointJson {
  /// Projective x-coordinate over Fq2.
  pub x: [String; 2],
  /// Projective y-coordinate over Fq2.
  pub y: [String; 2],
  /// Projective z-coordinate over Fq2.
  pub z: [String; 2],
}

impl PlannedGroth16G2PointJson {
  /// Builds a placeholder G2 point from a label stem.
  #[must_use]
  pub fn placeholder(label: impl Into<String>) -> Self {
    let label = label.into();
    Self {
      x: [format!("<{}-x-c0>", label), format!("<{}-x-c1>", label)],
      y: [format!("<{}-y-c0>", label), format!("<{}-y-c1>", label)],
      z: [format!("<{}-z-c0>", label), format!("<{}-z-c1>", label)],
    }
  }
}

/// Planned JSON payload for the future outer Groth16 proof artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedOuterGroth16ProofJson {
  /// Protocol label.
  pub protocol: String,
  /// Curve label.
  pub curve: String,
  /// Proof point `A`.
  pub pi_a: PlannedGroth16G1PointJson,
  /// Proof point `B`.
  pub pi_b: PlannedGroth16G2PointJson,
  /// Proof point `C`.
  pub pi_c: PlannedGroth16G1PointJson,
}

impl PlannedOuterGroth16ProofJson {
  /// Builds a placeholder outer-proof payload.
  #[must_use]
  pub fn placeholder(protocol: impl Into<String>, curve: impl Into<String>) -> Self {
    Self {
      protocol: protocol.into(),
      curve: curve.into(),
      pi_a: PlannedGroth16G1PointJson::placeholder("outer-proof-pi-a"),
      pi_b: PlannedGroth16G2PointJson::placeholder("outer-proof-pi-b"),
      pi_c: PlannedGroth16G1PointJson::placeholder("outer-proof-pi-c"),
    }
  }
}

/// Planned JSON payload for the future outer Groth16 verification-key artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedOuterGroth16VerificationKeyJson {
  /// Protocol label.
  pub protocol: String,
  /// Curve label.
  pub curve: String,
  /// Number of public inputs.
  #[serde(rename = "nPublic")]
  pub n_public: usize,
  /// Verification-key point `alpha`.
  #[serde(rename = "vk_alpha_1")]
  pub vk_alpha_1: PlannedGroth16G1PointJson,
  /// Verification-key point `beta`.
  #[serde(rename = "vk_beta_2")]
  pub vk_beta_2: PlannedGroth16G2PointJson,
  /// Verification-key point `gamma`.
  #[serde(rename = "vk_gamma_2")]
  pub vk_gamma_2: PlannedGroth16G2PointJson,
  /// Verification-key point `delta`.
  #[serde(rename = "vk_delta_2")]
  pub vk_delta_2: PlannedGroth16G2PointJson,
  /// IC table with the expected Groth16 arity relation.
  #[serde(rename = "IC")]
  pub ic: Vec<PlannedGroth16G1PointJson>,
}

impl PlannedOuterGroth16VerificationKeyJson {
  /// Builds a placeholder outer verification-key payload.
  #[must_use]
  pub fn placeholder(
    protocol: impl Into<String>,
    curve: impl Into<String>,
    public_input_count: usize,
  ) -> Self {
    Self {
      protocol: protocol.into(),
      curve: curve.into(),
      n_public: public_input_count,
      vk_alpha_1: PlannedGroth16G1PointJson::placeholder("outer-vk-alpha-1"),
      vk_beta_2: PlannedGroth16G2PointJson::placeholder("outer-vk-beta-2"),
      vk_gamma_2: PlannedGroth16G2PointJson::placeholder("outer-vk-gamma-2"),
      vk_delta_2: PlannedGroth16G2PointJson::placeholder("outer-vk-delta-2"),
      ic: (0..=public_input_count)
        .map(|index| PlannedGroth16G1PointJson::placeholder(format!("outer-vk-ic-{index}")))
        .collect(),
    }
  }
}

/// Planned/materialized outer Groth16 artifact bundle before real proving exists.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlannedOuterGroth16ArtifactBundle {
  /// Output proof system represented by the bundle.
  pub proof_system: ProofSystemDescriptor,
  /// Canonical circuit identity tied to this bundle when available.
  pub canonical_circuit_identity: Option<CanonicalCircuitIdentity>,
  /// Logical proof artifact identifier.
  pub proof_artifact: String,
  /// Materialized proof payload when available.
  pub proof: Option<PlannedOuterGroth16ProofJson>,
  /// Logical public-input artifact identifier.
  pub public_inputs_artifact: String,
  /// Planned public-input payload.
  pub public_inputs: Vec<String>,
  /// Logical verification-key artifact identifier.
  pub verification_key_artifact: String,
  /// Materialized verification-key payload when available.
  pub verification_key: Option<PlannedOuterGroth16VerificationKeyJson>,
  /// Notes about materialization state.
  pub notes: Vec<String>,
}

impl PlannedOuterGroth16ArtifactBundle {
  /// Builds an outer artifact bundle from explicit payloads.
  #[must_use]
  pub fn new(
    proof_system: ProofSystemDescriptor,
    canonical_circuit_identity: Option<CanonicalCircuitIdentity>,
    proof_artifact: impl Into<String>,
    proof: Option<PlannedOuterGroth16ProofJson>,
    public_inputs_artifact: impl Into<String>,
    public_inputs: Vec<String>,
    verification_key_artifact: impl Into<String>,
    verification_key: Option<PlannedOuterGroth16VerificationKeyJson>,
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
    statement: &WrapperStatement,
  ) -> Self {
    let identifier = identifier.into();
    let protocol = protocol.into();
    let curve = curve.into();
    Self {
      proof_system: ProofSystemDescriptor {
        kind: crate::ProofSystemKind::Groth16Bls12_381,
        source: "planned-groth16-bls12-381-wrapper".to_owned(),
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
      verification_key: Some(PlannedOuterGroth16VerificationKeyJson::placeholder(
        protocol.clone(),
        curve.clone(),
        statement.public_inputs.entries.len(),
      )),
      notes: vec![
        format!("planned {protocol}/{curve} outer bundle preserves the wrapper public inputs"),
        "proof payload remains absent until a real outer prover exists".to_owned(),
        "verification-key payload is materialized as a skeleton with placeholder coordinates"
          .to_owned(),
      ],
    }
  }
}

/// Produced G1 point payload using the final `snarkjs`-like projective JSON shape.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProducedGroth16G1PointJson {
  /// Projective x-coordinate.
  pub x: String,
  /// Projective y-coordinate.
  pub y: String,
  /// Projective z-coordinate.
  pub z: String,
}

/// Produced G2 point payload using the final `snarkjs`-like projective JSON shape.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProducedGroth16G2PointJson {
  /// Projective x-coordinate over Fq2.
  pub x: [String; 2],
  /// Projective y-coordinate over Fq2.
  pub y: [String; 2],
  /// Projective z-coordinate over Fq2.
  pub z: [String; 2],
}

/// Produced JSON payload for a real outer Groth16 proof artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProducedOuterGroth16ProofJson {
  /// Protocol label.
  pub protocol: String,
  /// Curve label.
  pub curve: String,
  /// Proof point `A`.
  pub pi_a: ProducedGroth16G1PointJson,
  /// Proof point `B`.
  pub pi_b: ProducedGroth16G2PointJson,
  /// Proof point `C`.
  pub pi_c: ProducedGroth16G1PointJson,
}

/// Produced JSON payload for a real outer Groth16 verification-key artifact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProducedOuterGroth16VerificationKeyJson {
  /// Protocol label.
  pub protocol: String,
  /// Curve label.
  pub curve: String,
  /// Number of public inputs.
  #[serde(rename = "nPublic")]
  pub n_public: usize,
  /// Verification-key point `alpha`.
  #[serde(rename = "vk_alpha_1")]
  pub vk_alpha_1: ProducedGroth16G1PointJson,
  /// Verification-key point `beta`.
  #[serde(rename = "vk_beta_2")]
  pub vk_beta_2: ProducedGroth16G2PointJson,
  /// Verification-key point `gamma`.
  #[serde(rename = "vk_gamma_2")]
  pub vk_gamma_2: ProducedGroth16G2PointJson,
  /// Verification-key point `delta`.
  #[serde(rename = "vk_delta_2")]
  pub vk_delta_2: ProducedGroth16G2PointJson,
  /// IC table with the expected Groth16 arity relation.
  #[serde(rename = "IC")]
  pub ic: Vec<ProducedGroth16G1PointJson>,
}

/// Strict produced outer Groth16 artifact bundle emitted by a real backend.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProducedOuterGroth16ArtifactBundle {
  /// Output proof system represented by the bundle.
  pub proof_system: ProofSystemDescriptor,
  /// Canonical circuit identity tied to this bundle when available.
  pub canonical_circuit_identity: Option<CanonicalCircuitIdentity>,
  /// Logical proof artifact identifier.
  pub proof_artifact: String,
  /// Produced proof payload.
  pub proof: ProducedOuterGroth16ProofJson,
  /// Logical public-input artifact identifier.
  pub public_inputs_artifact: String,
  /// Produced public-input payload.
  pub public_inputs: Vec<String>,
  /// Logical verification-key artifact identifier.
  pub verification_key_artifact: String,
  /// Produced verification-key payload.
  pub verification_key: ProducedOuterGroth16VerificationKeyJson,
  /// Notes about production/setup state.
  pub notes: Vec<String>,
}

impl ProducedOuterGroth16ArtifactBundle {
  /// Builds a produced outer artifact bundle from explicit payloads.
  #[must_use]
  pub fn new(
    proof_system: ProofSystemDescriptor,
    canonical_circuit_identity: Option<CanonicalCircuitIdentity>,
    proof_artifact: impl Into<String>,
    proof: ProducedOuterGroth16ProofJson,
    public_inputs_artifact: impl Into<String>,
    public_inputs: Vec<String>,
    verification_key_artifact: impl Into<String>,
    verification_key: ProducedOuterGroth16VerificationKeyJson,
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
  /// Top-level JSON keys expected in the proof artifact.
  pub top_level_keys: Vec<String>,
  /// Expected key for the first proof point.
  pub pi_a_key: String,
  /// Expected key for the second proof point.
  pub pi_b_key: String,
  /// Expected key for the third proof point.
  pub pi_c_key: String,
  /// Expected point encoding for G1 proof points.
  pub g1_point_encoding: String,
  /// Expected point encoding for the G2 proof point.
  pub g2_point_encoding: String,
  /// Whether the field naming is intentionally `snarkjs`-like.
  pub snarkjs_like_naming: bool,
}

impl ExpectedProofArtifactShape {
  /// Builds the expected proof artifact shape.
  #[must_use]
  pub fn new(
    format: impl Into<String>,
    protocol: impl Into<String>,
    curve: impl Into<String>,
    top_level_keys: Vec<String>,
    pi_a_key: impl Into<String>,
    pi_b_key: impl Into<String>,
    pi_c_key: impl Into<String>,
    g1_point_encoding: impl Into<String>,
    g2_point_encoding: impl Into<String>,
    snarkjs_like_naming: bool,
  ) -> Self {
    Self {
      format: format.into(),
      protocol: protocol.into(),
      curve: curve.into(),
      top_level_keys,
      pi_a_key: pi_a_key.into(),
      pi_b_key: pi_b_key.into(),
      pi_c_key: pi_c_key.into(),
      g1_point_encoding: g1_point_encoding.into(),
      g2_point_encoding: g2_point_encoding.into(),
      snarkjs_like_naming,
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
  /// Top-level JSON keys expected in the verification-key artifact.
  pub top_level_keys: Vec<String>,
  /// Expected key holding the public-input count.
  pub n_public_key: String,
  /// Expected key holding the IC table.
  pub ic_key: String,
  /// Expected point encoding for G1 verification-key points.
  pub g1_point_encoding: String,
  /// Expected point encoding for G2 verification-key points.
  pub g2_point_encoding: String,
  /// Expected relation between public-input count and IC table size.
  pub ic_shape_rule: String,
  /// Whether the field naming is intentionally `snarkjs`-like.
  pub snarkjs_like_naming: bool,
}

impl ExpectedVerificationKeyArtifactShape {
  /// Builds the expected verification-key artifact shape.
  #[must_use]
  pub fn new(
    format: impl Into<String>,
    protocol: impl Into<String>,
    curve: impl Into<String>,
    top_level_keys: Vec<String>,
    n_public_key: impl Into<String>,
    ic_key: impl Into<String>,
    g1_point_encoding: impl Into<String>,
    g2_point_encoding: impl Into<String>,
    ic_shape_rule: impl Into<String>,
    snarkjs_like_naming: bool,
  ) -> Self {
    Self {
      format: format.into(),
      protocol: protocol.into(),
      curve: curve.into(),
      top_level_keys,
      n_public_key: n_public_key.into(),
      ic_key: ic_key.into(),
      g1_point_encoding: g1_point_encoding.into(),
      g2_point_encoding: g2_point_encoding.into(),
      ic_shape_rule: ic_shape_rule.into(),
      snarkjs_like_naming,
    }
  }
}

/// Expected Groth16 wrapper artifacts once a real outer prover exists.
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
  pub bundle_template: PlannedOuterGroth16ArtifactBundle,
  /// Planning notes about the expected outputs.
  pub notes: Vec<String>,
}

impl ExpectedWrapperArtifacts {
  /// Builds the expected output artifact description.
  #[must_use]
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
    bundle_template: PlannedOuterGroth16ArtifactBundle,
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
    NamedPublicInputs, PlannedOuterGroth16ArtifactBundle, ProducedGroth16G1PointJson,
    ProducedGroth16G2PointJson, ProducedOuterGroth16ArtifactBundle, ProducedOuterGroth16ProofJson,
    ProducedOuterGroth16VerificationKeyJson, ProofSystemDescriptor, ProofSystemKind,
    WrapperStatement,
  };

  #[test]
  fn expected_wrapper_artifacts_keep_statement_shape() {
    let artifacts = ExpectedWrapperArtifacts::new(
      ProofSystemDescriptor {
        kind: ProofSystemKind::Groth16Bls12_381,
        source: "planner".to_owned(),
      },
      None,
      "proof.json",
      ExpectedProofArtifactShape::new(
        "json",
        "groth16",
        "bls12-381",
        vec![
          "pi_a".to_owned(),
          "pi_b".to_owned(),
          "pi_c".to_owned(),
          "protocol".to_owned(),
          "curve".to_owned(),
        ],
        "pi_a",
        "pi_b",
        "pi_c",
        "projective [x, y, z] decimal-string array",
        "projective [[x.c0, x.c1], [y.c0, y.c1], [z.c0, z.c1]] decimal-string array",
        true,
      ),
      "public.json",
      ExpectedPublicInputsArtifactShape::new("json", "array", "decimal-string"),
      "vk.json",
      ExpectedVerificationKeyArtifactShape::new(
        "json",
        "groth16",
        "bls12-381",
        vec![
          "protocol".to_owned(),
          "curve".to_owned(),
          "nPublic".to_owned(),
          "vk_alpha_1".to_owned(),
          "vk_beta_2".to_owned(),
          "vk_gamma_2".to_owned(),
          "vk_delta_2".to_owned(),
          "IC".to_owned(),
        ],
        "nPublic",
        "IC",
        "projective [x, y, z] decimal-string array",
        "projective [[x.c0, x.c1], [y.c0, y.c1], [z.c0, z.c1]] decimal-string array",
        "ic.len() == public_inputs.len() + 1",
        true,
      ),
      WrapperStatement::new(NamedPublicInputs::new(vec![
        NamedPublicInput::new("x", "1"),
        NamedPublicInput::new("y", "2"),
      ])),
      PlannedOuterGroth16ArtifactBundle::placeholder(
        "test-artifact",
        "groth16",
        "bls12-381",
        &WrapperStatement::new(NamedPublicInputs::new(vec![
          NamedPublicInput::new("x", "1"),
          NamedPublicInput::new("y", "2"),
        ])),
      ),
      vec![],
    );

    assert_eq!(artifacts.statement.public_inputs.field_order(), vec!["x", "y"]);
    assert_eq!(artifacts.public_inputs_shape.container, "array");
    assert_eq!(artifacts.canonical_circuit_identity, None);
    assert_eq!(artifacts.verification_key_shape.protocol, "groth16");
    assert_eq!(artifacts.proof_shape.pi_a_key, "pi_a");
    assert_eq!(artifacts.verification_key_shape.ic_key, "IC");
    assert!(artifacts.proof_shape.snarkjs_like_naming);
    assert_eq!(artifacts.bundle_template.public_inputs, vec!["1", "2"]);
    assert_eq!(artifacts.bundle_template.canonical_circuit_identity, None);
    assert!(artifacts.bundle_template.proof.is_none());
    assert_eq!(
      artifacts
        .bundle_template
        .verification_key
        .as_ref()
        .expect("bundle template should materialize a VK skeleton")
        .n_public,
      2
    );
    assert_eq!(
      artifacts
        .bundle_template
        .verification_key
        .as_ref()
        .expect("bundle template should materialize a VK skeleton")
        .ic
        .len(),
      3
    );
    assert_eq!(artifacts.bundle_template.proof_system.kind, ProofSystemKind::Groth16Bls12_381);
  }

  #[test]
  fn produced_outer_bundle_requires_real_proof_and_verification_key() {
    let bundle = ProducedOuterGroth16ArtifactBundle::new(
      ProofSystemDescriptor {
        kind: ProofSystemKind::Groth16Bls12_381,
        source: "real-backend".to_owned(),
      },
      Some(CanonicalCircuitIdentity::new("r1cs-blake2b-256", "deadbeef")),
      "proof.json",
      ProducedOuterGroth16ProofJson {
        protocol: "groth16".to_owned(),
        curve: "bls12-381".to_owned(),
        pi_a: ProducedGroth16G1PointJson {
          x: "1".to_owned(),
          y: "2".to_owned(),
          z: "1".to_owned(),
        },
        pi_b: ProducedGroth16G2PointJson {
          x: ["1".to_owned(), "0".to_owned()],
          y: ["2".to_owned(), "0".to_owned()],
          z: ["1".to_owned(), "0".to_owned()],
        },
        pi_c: ProducedGroth16G1PointJson {
          x: "3".to_owned(),
          y: "4".to_owned(),
          z: "1".to_owned(),
        },
      },
      "public.json",
      vec!["1".to_owned(), "2".to_owned()],
      "vk.json",
      ProducedOuterGroth16VerificationKeyJson {
        protocol: "groth16".to_owned(),
        curve: "bls12-381".to_owned(),
        n_public: 2,
        vk_alpha_1: ProducedGroth16G1PointJson {
          x: "1".to_owned(),
          y: "2".to_owned(),
          z: "1".to_owned(),
        },
        vk_beta_2: ProducedGroth16G2PointJson {
          x: ["1".to_owned(), "0".to_owned()],
          y: ["2".to_owned(), "0".to_owned()],
          z: ["1".to_owned(), "0".to_owned()],
        },
        vk_gamma_2: ProducedGroth16G2PointJson {
          x: ["3".to_owned(), "0".to_owned()],
          y: ["4".to_owned(), "0".to_owned()],
          z: ["1".to_owned(), "0".to_owned()],
        },
        vk_delta_2: ProducedGroth16G2PointJson {
          x: ["5".to_owned(), "0".to_owned()],
          y: ["6".to_owned(), "0".to_owned()],
          z: ["1".to_owned(), "0".to_owned()],
        },
        ic: vec![
          ProducedGroth16G1PointJson { x: "1".to_owned(), y: "2".to_owned(), z: "1".to_owned() },
          ProducedGroth16G1PointJson { x: "3".to_owned(), y: "4".to_owned(), z: "1".to_owned() },
          ProducedGroth16G1PointJson { x: "5".to_owned(), y: "6".to_owned(), z: "1".to_owned() },
        ],
      },
      vec![],
    );

    assert_eq!(bundle.proof.protocol, "groth16");
    assert_eq!(
      bundle.canonical_circuit_identity,
      Some(CanonicalCircuitIdentity::new("r1cs-blake2b-256", "deadbeef"))
    );
    assert_eq!(bundle.verification_key.n_public, 2);
    assert_eq!(bundle.verification_key.ic.len(), 3);
  }
}
