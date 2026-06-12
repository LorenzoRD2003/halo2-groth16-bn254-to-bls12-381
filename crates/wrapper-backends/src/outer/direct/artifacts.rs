use wrapper_core::{
  ProducedOuterProofArtifactBundle, ProducedOuterProofJson, ProducedOuterVerificationKeyJson,
  WrapperExecutionPackage,
};

use super::{MidnightDirectOuterBackendBls12Host, MidnightDirectOuterBackendBn254Host};
use crate::outer::{OuterProofBackend, OuterProofBackendError};

macro_rules! impl_direct_backend_artifacts {
  ($backend:ty) => {
impl $backend {
  /// Validates that one produced setup verification key matches the current
  /// wrapper-core planning contract.
  ///
  /// # Errors
  ///
  /// Returns an error if the verification key does not match the expected
  /// protocol, curve, arity, or top-level field layout.
  pub fn validate_setup_verification_key(
    &self,
    package: &WrapperExecutionPackage,
    verification_key: &ProducedOuterVerificationKeyJson,
  ) -> Result<(), OuterProofBackendError> {
    let planned = self.prepare(package)?;
    let expected_shape = planned.verification_key_shape;

    if verification_key.protocol != expected_shape.protocol {
      return Err(OuterProofBackendError::VerificationKeyProtocolMismatch {
        expected: expected_shape.protocol,
        actual: verification_key.protocol.clone(),
      });
    }

    if verification_key.curve != expected_shape.curve {
      return Err(OuterProofBackendError::VerificationKeyCurveMismatch {
        expected: expected_shape.curve,
        actual: verification_key.curve.clone(),
      });
    }

    if verification_key.public_input_count != package.statement.public_inputs.entries.len() {
      return Err(OuterProofBackendError::VerificationKeyPublicInputCountMismatch {
        expected: package.statement.public_inputs.entries.len(),
        actual: verification_key.public_input_count,
      });
    }

    let serialized = serde_json::to_value(verification_key).expect("produced VK should serialize");
    let mut actual_keys = serialized
      .as_object()
      .expect("produced VK should serialize as a JSON object")
      .keys()
      .cloned()
      .collect::<Vec<_>>();
    let mut expected_keys = expected_shape.top_level_keys;
    actual_keys.sort();
    expected_keys.sort();

    if actual_keys != expected_keys {
      return Err(OuterProofBackendError::VerificationKeyTopLevelKeysMismatch {
        expected: expected_keys,
        actual: actual_keys,
      });
    }

    Ok(())
  }

  /// Validates that one produced outer proof matches the current wrapper-core
  /// planning contract.
  ///
  /// # Errors
  ///
  /// Returns an error if the proof does not match the expected protocol, curve,
  /// or top-level field layout.
  pub fn validate_produced_proof(
    &self,
    package: &WrapperExecutionPackage,
    proof: &ProducedOuterProofJson,
  ) -> Result<(), OuterProofBackendError> {
    let planned = self.prepare(package)?;
    let expected_shape = planned.proof_shape;

    if proof.protocol != expected_shape.protocol {
      return Err(OuterProofBackendError::ProofProtocolMismatch {
        expected: expected_shape.protocol,
        actual: proof.protocol.clone(),
      });
    }

    if proof.curve != expected_shape.curve {
      return Err(OuterProofBackendError::ProofCurveMismatch {
        expected: expected_shape.curve,
        actual: proof.curve.clone(),
      });
    }

    let serialized = serde_json::to_value(proof).expect("produced proof should serialize");
    let mut actual_keys = serialized
      .as_object()
      .expect("produced proof should serialize as a JSON object")
      .keys()
      .cloned()
      .collect::<Vec<_>>();
    let mut expected_keys = expected_shape.top_level_keys;
    actual_keys.sort();
    expected_keys.sort();

    if actual_keys != expected_keys {
      return Err(OuterProofBackendError::ProofTopLevelKeysMismatch {
        expected: expected_keys,
        actual: actual_keys,
      });
    }

    Ok(())
  }

  /// Assembles a strict produced outer artifact bundle from validated proof and
  /// verification-key payloads.
  ///
  /// # Errors
  ///
  /// Returns an error if either the proof or verification key violates the
  /// current planning contract.
  pub fn assemble_produced_bundle(
    &self,
    package: &WrapperExecutionPackage,
    proof: ProducedOuterProofJson,
    verification_key: ProducedOuterVerificationKeyJson,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError> {
    let planned = self.prepare(package)?;
    self.validate_produced_proof(package, &proof)?;
    self.validate_setup_verification_key(package, &verification_key)?;

    Ok(ProducedOuterProofArtifactBundle::new(
      planned.proof_system,
      planned.canonical_circuit_identity,
      planned.proof_artifact,
      proof,
      planned.public_inputs_artifact,
      package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| entry.value.clone())
        .collect(),
      planned.verification_key_artifact,
      verification_key,
      vec![
        format!("selected outer backend stack: {}", self.metadata().stack),
        "produced bundle matches the current wrapper-core proof and verification-key shape contracts"
          .to_owned(),
      ],
    ))
  }
}
  };
}

impl_direct_backend_artifacts!(MidnightDirectOuterBackendBn254Host);
impl_direct_backend_artifacts!(MidnightDirectOuterBackendBls12Host);
