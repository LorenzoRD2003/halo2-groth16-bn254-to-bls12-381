//! Serializable wrapper package types for future executors.

use serde::{Deserialize, Serialize};

use crate::{NamedPublicInputs, ProofSystemDescriptor, WrapperJob};

/// Public statement exposed by the planned outer wrapper.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WrapperStatement {
  /// Ordered public statement fields.
  pub public_inputs: NamedPublicInputs,
}

impl WrapperStatement {
  /// Builds a wrapper statement from named public inputs.
  #[must_use]
  pub fn new(public_inputs: NamedPublicInputs) -> Self {
    Self { public_inputs }
  }
}

/// Witness-oriented metadata required by a future wrapper executor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WrapperWitnessInput {
  /// Logical identifier of the source artifact bundle.
  pub source_artifact_id: String,
  /// Source proof system expected by the wrapper executor.
  pub source_proof_system: ProofSystemDescriptor,
  /// Ordered verifier public inputs seen by the inner proof verifier.
  pub verifier_public_inputs: NamedPublicInputs,
  /// Number of IC points present in the inner verification key.
  pub verification_key_ic_count: usize,
  /// Whether a proof payload is expected alongside this package.
  pub requires_inner_proof: bool,
  /// Whether a verification-key payload is expected alongside this package.
  pub requires_verification_key: bool,
  /// Executor-facing notes about current-stage limits.
  pub notes: Vec<String>,
}

impl WrapperWitnessInput {
  /// Builds wrapper witness input metadata.
  #[must_use]
  pub fn new(
    source_artifact_id: impl Into<String>,
    source_proof_system: ProofSystemDescriptor,
    verifier_public_inputs: NamedPublicInputs,
    verification_key_ic_count: usize,
    requires_inner_proof: bool,
    requires_verification_key: bool,
    notes: Vec<String>,
  ) -> Self {
    Self {
      source_artifact_id: source_artifact_id.into(),
      source_proof_system,
      verifier_public_inputs,
      verification_key_ic_count,
      requires_inner_proof,
      requires_verification_key,
      notes,
    }
  }
}

/// Full serializable package handed to a future wrapper executor.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WrapperExecutionPackage {
  /// Planned wrapper job metadata.
  pub job: WrapperJob,
  /// Outer wrapper public statement.
  pub statement: WrapperStatement,
  /// Witness-oriented executor input metadata.
  pub witness: WrapperWitnessInput,
}

impl WrapperExecutionPackage {
  /// Builds a wrapper execution package.
  #[must_use]
  pub fn new(job: WrapperJob, statement: WrapperStatement, witness: WrapperWitnessInput) -> Self {
    Self { job, statement, witness }
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    NamedPublicInput, NamedPublicInputs, ProofSystemDescriptor, ProofSystemKind, WrapperExecutionPackage,
    WrapperJob, WrapperStatement, WrapperWitnessInput,
  };

  #[test]
  fn wrapper_execution_package_preserves_statement_and_witness_order() {
    let named = NamedPublicInputs::new(vec![
      NamedPublicInput::new("a", "1"),
      NamedPublicInput::new("b", "2"),
    ]);
    let job = WrapperJob::new(
      "job-1",
      ProofSystemDescriptor {
        kind: ProofSystemKind::Groth16Bn254,
        source: "loader".to_owned(),
      },
      ProofSystemDescriptor {
        kind: ProofSystemKind::Groth16Bls12_381,
        source: "planner".to_owned(),
      },
      2,
      Some(named.clone()),
      vec![],
    );
    let package = WrapperExecutionPackage::new(
      job,
      WrapperStatement::new(named.clone()),
      WrapperWitnessInput::new(
        "artifact-1",
        ProofSystemDescriptor {
          kind: ProofSystemKind::Groth16Bn254,
          source: "loader".to_owned(),
        },
        named,
        3,
        true,
        true,
        vec![],
      ),
    );

    assert_eq!(package.statement.public_inputs.field_order(), vec!["a", "b"]);
    assert_eq!(package.witness.verifier_public_inputs.field_order(), vec!["a", "b"]);
  }
}
