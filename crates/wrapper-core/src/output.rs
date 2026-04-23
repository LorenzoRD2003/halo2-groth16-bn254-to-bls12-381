//! Expected wrapper output shapes.

use serde::{Deserialize, Serialize};

use crate::{ProofSystemDescriptor, WrapperStatement};

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
  /// Planning notes about the expected outputs.
  pub notes: Vec<String>,
}

impl ExpectedWrapperArtifacts {
  /// Builds the expected output artifact description.
  #[must_use]
  pub fn new(
    proof_system: ProofSystemDescriptor,
    proof_artifact: impl Into<String>,
    proof_shape: ExpectedProofArtifactShape,
    public_inputs_artifact: impl Into<String>,
    public_inputs_shape: ExpectedPublicInputsArtifactShape,
    verification_key_artifact: impl Into<String>,
    verification_key_shape: ExpectedVerificationKeyArtifactShape,
    statement: WrapperStatement,
    notes: Vec<String>,
  ) -> Self {
    Self {
      proof_system,
      proof_artifact: proof_artifact.into(),
      proof_shape,
      public_inputs_artifact: public_inputs_artifact.into(),
      public_inputs_shape,
      verification_key_artifact: verification_key_artifact.into(),
      verification_key_shape,
      statement,
      notes,
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    ExpectedProofArtifactShape, ExpectedPublicInputsArtifactShape,
    ExpectedVerificationKeyArtifactShape, ExpectedWrapperArtifacts, NamedPublicInput,
    NamedPublicInputs, ProofSystemDescriptor, ProofSystemKind, WrapperStatement,
  };

  #[test]
  fn expected_wrapper_artifacts_keep_statement_shape() {
    let artifacts = ExpectedWrapperArtifacts::new(
      ProofSystemDescriptor {
        kind: ProofSystemKind::Groth16Bls12_381,
        source: "planner".to_owned(),
      },
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
      vec![],
    );

    assert_eq!(artifacts.statement.public_inputs.field_order(), vec!["x", "y"]);
    assert_eq!(artifacts.public_inputs_shape.container, "array");
    assert_eq!(artifacts.verification_key_shape.protocol, "groth16");
    assert_eq!(artifacts.proof_shape.pi_a_key, "pi_a");
    assert_eq!(artifacts.verification_key_shape.ic_key, "IC");
    assert!(artifacts.proof_shape.snarkjs_like_naming);
  }
}
