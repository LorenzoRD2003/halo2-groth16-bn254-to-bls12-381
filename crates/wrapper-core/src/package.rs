//! Serializable wrapper package types for future executors.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{NamedPublicInputs, ProofSystemDescriptor, WrapperJob};

/// Public commitment to the inner verification key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerificationKeyCommitment {
  /// Semantic field name for the commitment.
  pub field_name: String,
  /// Canonical commitment value encoded as text.
  pub value: String,
  /// Flattened public-input projection used by the current outer lane.
  pub public_inputs: NamedPublicInputs,
}

impl VerificationKeyCommitment {
  /// Builds one explicit verification-key commitment component.
  #[must_use]
  pub fn new(
    field_name: impl Into<String>,
    value: impl Into<String>,
    public_inputs: NamedPublicInputs,
  ) -> Self {
    Self { field_name: field_name.into(), value: value.into(), public_inputs }
  }

  /// Returns the flattened public-input arity of the commitment projection.
  #[must_use]
  pub fn public_input_count(&self) -> usize {
    self.public_inputs.entries.len()
  }
}

/// Public statement exposed by the planned outer wrapper.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WrapperStatement {
  /// Ordered mirrored inner public inputs.
  pub mirrored_public_inputs: NamedPublicInputs,
  /// Explicit public verification-key commitment.
  pub vk_commitment: VerificationKeyCommitment,
  /// Ordered flat public statement fields exposed by the current outer lane.
  pub public_inputs: NamedPublicInputs,
}

impl WrapperStatement {
  /// Builds a wrapper statement from explicit semantic components.
  #[must_use]
  pub fn new(
    mirrored_public_inputs: NamedPublicInputs,
    vk_commitment: VerificationKeyCommitment,
  ) -> Self {
    let mut flattened = mirrored_public_inputs.entries.clone();
    flattened.extend(vk_commitment.public_inputs.entries.clone());

    Self { mirrored_public_inputs, vk_commitment, public_inputs: NamedPublicInputs::new(flattened) }
  }

  /// Returns the number of ordered public inputs in the statement.
  #[must_use]
  pub fn public_input_count(&self) -> usize {
    self.public_inputs.entries.len()
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

  /// Returns the number of ordered verifier public inputs in the witness view.
  #[must_use]
  pub fn verifier_public_input_count(&self) -> usize {
    self.verifier_public_inputs.entries.len()
  }
}

/// Canonical rule for how the outer wrapper statement relates to the inner proof statement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OuterStatementSemantics {
  /// The outer statement mirrors inner public inputs and binds one VK commitment.
  MirrorInnerVerifierPublicInputsAndVerificationKeyCommitment,
}

/// Canonical outer-statement contract derived from a wrapper execution package.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OuterStatementContract {
  /// Generic semantics rule applied by the wrapper lane.
  pub semantics: OuterStatementSemantics,
  /// Expected ordered outer public-input arity.
  pub expected_outer_public_input_count: usize,
  /// Expected inner verifier public-input arity.
  pub expected_inner_public_input_count: usize,
  /// Expected flattened public-input arity for the VK commitment projection.
  pub expected_vk_commitment_public_input_count: usize,
  /// Expected verification-key IC arity.
  pub expected_verification_key_ic_count: usize,
}

impl OuterStatementContract {
  /// Builds the canonical current-stage contract.
  #[must_use]
  pub fn mirrored_public_inputs_with_vk_commitment(
    public_input_count: usize,
    vk_commitment_public_input_count: usize,
  ) -> Self {
    Self {
      semantics:
        OuterStatementSemantics::MirrorInnerVerifierPublicInputsAndVerificationKeyCommitment,
      expected_outer_public_input_count: public_input_count + vk_commitment_public_input_count,
      expected_inner_public_input_count: public_input_count,
      expected_vk_commitment_public_input_count: vk_commitment_public_input_count,
      expected_verification_key_ic_count: public_input_count + 1,
    }
  }
}

/// Errors raised when the outer-statement contract is violated.
#[derive(Clone, Debug, Eq, PartialEq, Error, Serialize, Deserialize)]
pub enum OuterStatementContractError {
  /// The wrapper statement does not match the expected outer public-input arity.
  #[error("outer statement arity mismatch: expected {expected}, got {actual}")]
  OuterStatementArityMismatch {
    /// Expected arity from the package contract.
    expected: usize,
    /// Actual arity found in the wrapper statement.
    actual: usize,
  },
  /// The witness view over inner verifier public inputs has the wrong arity.
  #[error("inner verifier public-input arity mismatch: expected {expected}, got {actual}")]
  InnerVerifierPublicInputArityMismatch {
    /// Expected inner public-input arity from the package contract.
    expected: usize,
    /// Actual inner public-input arity found in the witness metadata.
    actual: usize,
  },
  /// The explicit mirrored statement shape drifted from the expected inner arity.
  #[error("mirrored public-input arity mismatch: expected {expected}, got {actual}")]
  MirroredPublicInputArityMismatch {
    /// Expected mirrored public-input arity.
    expected: usize,
    /// Actual mirrored public-input arity.
    actual: usize,
  },
  /// The wrapper statement field order does not mirror the inner verifier field order.
  #[error("outer statement field order does not mirror inner verifier public-input field order")]
  FieldOrderMismatch,
  /// The explicit VK commitment projection is missing or has the wrong shape.
  #[error(
    "verification-key commitment public-input arity mismatch: expected {expected}, got {actual}"
  )]
  VerificationKeyCommitmentArityMismatch {
    /// Expected flattened commitment arity.
    expected: usize,
    /// Actual flattened commitment arity.
    actual: usize,
  },
  /// The inner verification-key IC arity does not satisfy the Groth16 `n_public + 1` rule.
  #[error("inner verification-key IC arity mismatch: expected {expected}, got {actual}")]
  VerificationKeyIcArityMismatch {
    /// Expected IC arity from the package contract.
    expected: usize,
    /// Actual IC arity found in the witness metadata.
    actual: usize,
  },
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

  /// Returns the canonical outer-statement contract for the package.
  #[must_use]
  pub fn outer_statement_contract(&self) -> OuterStatementContract {
    OuterStatementContract::mirrored_public_inputs_with_vk_commitment(
      self.job.public_input_count,
      self.statement.vk_commitment.public_input_count(),
    )
  }

  /// Validates that the package respects the canonical outer-statement contract.
  ///
  /// # Errors
  ///
  /// Returns an error if the package no longer mirrors the inner verifier
  /// public inputs exactly or violates the Groth16 `IC.len() == n_public + 1`
  /// rule.
  pub fn validate_outer_statement_contract(
    &self,
  ) -> Result<OuterStatementContract, OuterStatementContractError> {
    let contract = self.outer_statement_contract();

    if self.statement.public_input_count() != contract.expected_outer_public_input_count {
      return Err(OuterStatementContractError::OuterStatementArityMismatch {
        expected: contract.expected_outer_public_input_count,
        actual: self.statement.public_input_count(),
      });
    }

    if self.witness.verifier_public_input_count() != contract.expected_inner_public_input_count {
      return Err(OuterStatementContractError::InnerVerifierPublicInputArityMismatch {
        expected: contract.expected_inner_public_input_count,
        actual: self.witness.verifier_public_input_count(),
      });
    }

    if self.statement.mirrored_public_inputs.entries.len()
      != contract.expected_inner_public_input_count
    {
      return Err(OuterStatementContractError::MirroredPublicInputArityMismatch {
        expected: contract.expected_inner_public_input_count,
        actual: self.statement.mirrored_public_inputs.entries.len(),
      });
    }

    if self.statement.vk_commitment.public_input_count()
      != contract.expected_vk_commitment_public_input_count
    {
      return Err(OuterStatementContractError::VerificationKeyCommitmentArityMismatch {
        expected: contract.expected_vk_commitment_public_input_count,
        actual: self.statement.vk_commitment.public_input_count(),
      });
    }

    if self.statement.mirrored_public_inputs.field_order()
      != self.witness.verifier_public_inputs.field_order()
    {
      return Err(OuterStatementContractError::FieldOrderMismatch);
    }

    if self.witness.verification_key_ic_count != contract.expected_verification_key_ic_count {
      return Err(OuterStatementContractError::VerificationKeyIcArityMismatch {
        expected: contract.expected_verification_key_ic_count,
        actual: self.witness.verification_key_ic_count,
      });
    }

    Ok(contract)
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    NamedPublicInput, NamedPublicInputs, ProofSystemDescriptor, ProofSystemKind,
    VerificationKeyCommitment, WrapperExecutionPackage, WrapperJob, WrapperStatement,
    WrapperWitnessInput, package::OuterStatementContractError,
  };

  fn sample_vk_commitment() -> VerificationKeyCommitment {
    VerificationKeyCommitment::new(
      "vk_commitment",
      "7",
      NamedPublicInputs::new(vec![
        NamedPublicInput::new("vk_commitment_limb_0", "7"),
        NamedPublicInput::new("vk_commitment_limb_1", "0"),
      ]),
    )
  }

  #[test]
  fn wrapper_execution_package_preserves_statement_and_witness_order() {
    let named = NamedPublicInputs::new(vec![
      NamedPublicInput::new("a", "1"),
      NamedPublicInput::new("b", "2"),
    ]);
    let job = WrapperJob::new(
      "job-1",
      ProofSystemDescriptor { kind: ProofSystemKind::Groth16Bn254, source: "loader".to_owned() },
      ProofSystemDescriptor { kind: ProofSystemKind::Halo2Outer, source: "planner".to_owned() },
      2,
      Some(named.clone()),
      vec![],
    );
    let package = WrapperExecutionPackage::new(
      job,
      WrapperStatement::new(named.clone(), sample_vk_commitment()),
      WrapperWitnessInput::new(
        "artifact-1",
        ProofSystemDescriptor { kind: ProofSystemKind::Groth16Bn254, source: "loader".to_owned() },
        named,
        3,
        true,
        true,
        vec![],
      ),
    );

    assert_eq!(
      package.statement.public_inputs.field_order(),
      vec!["a", "b", "vk_commitment_limb_0", "vk_commitment_limb_1"]
    );
    assert_eq!(package.witness.verifier_public_inputs.field_order(), vec!["a", "b"]);
  }

  fn sample_package() -> WrapperExecutionPackage {
    let named = NamedPublicInputs::new(vec![
      NamedPublicInput::new("a", "1"),
      NamedPublicInput::new("b", "2"),
    ]);

    WrapperExecutionPackage::new(
      WrapperJob::new(
        "job-1",
        ProofSystemDescriptor { kind: ProofSystemKind::Groth16Bn254, source: "loader".to_owned() },
        ProofSystemDescriptor { kind: ProofSystemKind::Halo2Outer, source: "planner".to_owned() },
        2,
        Some(named.clone()),
        vec![],
      ),
      WrapperStatement::new(named.clone(), sample_vk_commitment()),
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
  fn outer_statement_contract_accepts_mirrored_statement() {
    let package = sample_package();
    let contract = package
      .validate_outer_statement_contract()
      .expect("package should satisfy the canonical mirrored statement contract");

    assert_eq!(contract.expected_outer_public_input_count, 4);
    assert_eq!(contract.expected_inner_public_input_count, 2);
    assert_eq!(contract.expected_vk_commitment_public_input_count, 2);
    assert_eq!(contract.expected_verification_key_ic_count, 3);
  }

  #[test]
  fn outer_statement_contract_rejects_outer_statement_arity_mismatch() {
    let mut package = sample_package();
    package.statement = WrapperStatement::new(
      NamedPublicInputs::new(vec![NamedPublicInput::new("a", "1")]),
      sample_vk_commitment(),
    );

    assert_eq!(
      package.validate_outer_statement_contract(),
      Err(OuterStatementContractError::OuterStatementArityMismatch { expected: 4, actual: 3 })
    );
  }

  #[test]
  fn outer_statement_contract_rejects_inner_public_input_arity_mismatch() {
    let mut package = sample_package();
    package.witness.verifier_public_inputs =
      NamedPublicInputs::new(vec![NamedPublicInput::new("a", "1")]);

    assert_eq!(
      package.validate_outer_statement_contract(),
      Err(OuterStatementContractError::InnerVerifierPublicInputArityMismatch {
        expected: 2,
        actual: 1,
      })
    );
  }

  #[test]
  fn outer_statement_contract_rejects_field_order_drift() {
    let mut package = sample_package();
    package.statement = WrapperStatement::new(
      NamedPublicInputs::new(vec![
        NamedPublicInput::new("b", "1"),
        NamedPublicInput::new("a", "2"),
      ]),
      sample_vk_commitment(),
    );

    assert_eq!(
      package.validate_outer_statement_contract(),
      Err(OuterStatementContractError::FieldOrderMismatch)
    );
  }

  #[test]
  fn outer_statement_contract_tracks_verification_key_commitment_arity() {
    let mut package = sample_package();
    package.statement = WrapperStatement::new(
      package.statement.mirrored_public_inputs.clone(),
      VerificationKeyCommitment::new(
        "vk_commitment",
        "7",
        NamedPublicInputs::new(vec![NamedPublicInput::new("vk_commitment_limb_0", "7")]),
      ),
    );

    let contract = package
      .validate_outer_statement_contract()
      .expect("statement should remain self-consistent after a different commitment projection");
    assert_eq!(contract.expected_vk_commitment_public_input_count, 1);
  }

  #[test]
  fn outer_statement_contract_rejects_verification_key_ic_arity_mismatch() {
    let mut package = sample_package();
    package.witness.verification_key_ic_count = 4;

    assert_eq!(
      package.validate_outer_statement_contract(),
      Err(OuterStatementContractError::VerificationKeyIcArityMismatch { expected: 3, actual: 4 })
    );
  }
}
