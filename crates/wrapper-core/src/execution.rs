//! Wrapper execution result types and stub executor.

use serde::{Deserialize, Serialize};

use crate::{
  ExpectedProofArtifactShape, ExpectedPublicInputsArtifactShape,
  ExpectedVerificationKeyArtifactShape, ExpectedWrapperArtifacts,
  PlannedOuterGroth16ArtifactBundle, WrapperExecutionPackage,
};

/// High-level outcome of a wrapper execution attempt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WrapperExecutionStatus {
  /// Package validation succeeded, but real proof generation is not implemented yet.
  PlannedOnly,
  /// Package validation failed before execution could proceed.
  Rejected,
}

/// Structured result from a wrapper execution attempt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WrapperExecutionResult {
  /// Identifier of the executed job.
  pub job_id: String,
  /// Final high-level status.
  pub status: WrapperExecutionStatus,
  /// Whether the package passed stub precondition checks.
  pub preflight_ok: bool,
  /// Expected wrapper artifacts once a real executor exists.
  pub expected_output: Option<ExpectedWrapperArtifacts>,
  /// Human-readable execution notes.
  pub notes: Vec<String>,
}

impl WrapperExecutionResult {
  /// Builds a wrapper execution result.
  #[must_use]
  pub fn new(
    job_id: impl Into<String>,
    status: WrapperExecutionStatus,
    preflight_ok: bool,
    expected_output: Option<ExpectedWrapperArtifacts>,
    notes: Vec<String>,
  ) -> Self {
    Self { job_id: job_id.into(), status, preflight_ok, expected_output, notes }
  }
}

impl WrapperExecutionPackage {
  /// Describes the artifacts that a real wrapper executor is expected to emit.
  #[must_use]
  pub fn expected_output(&self) -> ExpectedWrapperArtifacts {
    ExpectedWrapperArtifacts::new(
      self.job.target.clone(),
      format!("{}-wrapper-proof.json", self.job.identifier),
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
      format!("{}-wrapper-public.json", self.job.identifier),
      ExpectedPublicInputsArtifactShape::new("json", "array", "decimal-string"),
      format!("{}-wrapper-verification-key.json", self.job.identifier),
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
      self.statement.clone(),
      PlannedOuterGroth16ArtifactBundle::placeholder(
        self.job.identifier.clone(),
        "groth16",
        "bls12-381",
        &self.statement,
      ),
      vec![
        "proof artifact shape is planned to follow a Groth16-style external bundle".to_owned(),
        "public-input artifact is planned to follow the current snarkjs-style JSON array convention".to_owned(),
        "verification key artifact is modeled as reusable across wrapper proofs for one circuit configuration".to_owned(),
      ],
    )
  }

  /// Runs the current stub executor over the package.
  #[must_use]
  pub fn execute_stub(&self) -> WrapperExecutionResult {
    let mut notes = Vec::new();

    if let Err(error) = self.validate_outer_statement_contract() {
      notes.push(error.to_string());
      return WrapperExecutionResult::new(
        self.job.identifier.clone(),
        WrapperExecutionStatus::Rejected,
        false,
        None,
        notes,
      );
    }

    if !self.witness.requires_inner_proof || !self.witness.requires_verification_key {
      notes.push(
        "wrapper witness input must require both inner proof and verification key".to_owned(),
      );
      return WrapperExecutionResult::new(
        self.job.identifier.clone(),
        WrapperExecutionStatus::Rejected,
        false,
        None,
        notes,
      );
    }

    notes.push("package preflight checks passed".to_owned());
    notes.push(
      "stub executor stops before outer proof synthesis in the current repository phase".to_owned(),
    );

    WrapperExecutionResult::new(
      self.job.identifier.clone(),
      WrapperExecutionStatus::PlannedOnly,
      true,
      Some(self.expected_output()),
      notes,
    )
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    NamedPublicInput, NamedPublicInputs, ProofSystemDescriptor, ProofSystemKind,
    WrapperExecutionPackage, WrapperExecutionStatus, WrapperJob, WrapperStatement,
    WrapperWitnessInput,
  };

  fn sample_package() -> WrapperExecutionPackage {
    let named = NamedPublicInputs::new(vec![
      NamedPublicInput::new("a", "1"),
      NamedPublicInput::new("b", "2"),
    ]);

    WrapperExecutionPackage::new(
      WrapperJob::new(
        "job-1",
        ProofSystemDescriptor { kind: ProofSystemKind::Groth16Bn254, source: "loader".to_owned() },
        ProofSystemDescriptor {
          kind: ProofSystemKind::Groth16Bls12_381,
          source: "planner".to_owned(),
        },
        2,
        Some(named.clone()),
        vec![],
      ),
      WrapperStatement::new(named.clone()),
      WrapperWitnessInput::new(
        "artifact-1",
        ProofSystemDescriptor { kind: ProofSystemKind::Groth16Bn254, source: "loader".to_owned() },
        named,
        3,
        true,
        true,
        vec![],
      ),
    )
  }

  #[test]
  fn stub_executor_accepts_consistent_package() {
    let result = sample_package().execute_stub();

    assert_eq!(result.status, WrapperExecutionStatus::PlannedOnly);
    assert!(result.preflight_ok);
    assert!(result.expected_output.is_some());
  }

  #[test]
  fn stub_executor_rejects_mismatched_public_input_count() {
    let mut package = sample_package();
    package.job.public_input_count = 3;

    let result = package.execute_stub();

    assert_eq!(result.status, WrapperExecutionStatus::Rejected);
    assert!(!result.preflight_ok);
  }

  #[test]
  fn stub_executor_rejects_mismatched_ic_count() {
    let mut package = sample_package();
    package.witness.verification_key_ic_count = 2;

    let result = package.execute_stub();

    assert_eq!(result.status, WrapperExecutionStatus::Rejected);
    assert!(!result.preflight_ok);
  }
}
