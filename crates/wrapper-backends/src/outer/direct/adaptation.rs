use wrapper_circuits::OuterWrapperCircuit;
use wrapper_core::WrapperExecutionPackage;

use crate::snarkjs::{parse_groth16_bn254_proof, parse_groth16_bn254_verifying_key};

use super::{MidnightDirectOuterBackend, MidnightDirectOuterBackendBls12Host};
use crate::outer::{
  DirectOuterCircuitInput, DirectOuterStatementInput, OuterCircuitInputArtifacts,
  OuterProofBackend, OuterProofBackendError, helpers::parse_native_input_value,
};

macro_rules! impl_direct_backend_adaptation {
  ($backend:ty) => {
impl $backend {
  /// Adapts a wrapper execution package plus raw inner artifacts into the exact
  /// normalized input shape expected by the selected arkworks outer lane.
  ///
  /// # Errors
  ///
  /// Returns an error if the package is invalid, required inner artifacts are
  /// missing or malformed, the parsed verification key disagrees with package
  /// arity metadata, or the outer statement no longer mirrors the inner
  /// verifier public-input values exactly.
  pub fn adapt_input(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<DirectOuterCircuitInput, OuterProofBackendError> {
    let _ = self.prepare(package)?;

    let proof_json =
      artifacts.proof_json.ok_or_else(|| OuterProofBackendError::MissingInnerProofPayload {
        source_artifact_id: package.witness.source_artifact_id.clone(),
      })?;
    let verification_key_json = artifacts.verification_key_json.ok_or_else(|| {
      OuterProofBackendError::MissingInnerVerificationKeyPayload {
        source_artifact_id: package.witness.source_artifact_id.clone(),
      }
    })?;

    let inner_proof = parse_groth16_bn254_proof(proof_json).map_err(|source| {
      OuterProofBackendError::MalformedInnerProof {
        source_artifact_id: package.witness.source_artifact_id.clone(),
        source,
      }
    })?;
    let inner_verification_key =
      parse_groth16_bn254_verifying_key(verification_key_json).map_err(|source| {
        OuterProofBackendError::MalformedInnerVerificationKey {
          source_artifact_id: package.witness.source_artifact_id.clone(),
          source,
        }
      })?;

    if inner_verification_key.ic.len() != package.witness.verification_key_ic_count {
      return Err(OuterProofBackendError::VerificationKeyIcCountMismatch {
        expected: package.witness.verification_key_ic_count,
        actual: inner_verification_key.ic.len(),
      });
    }

    let inner_verifier_public_inputs = package
      .witness
      .verifier_public_inputs
      .entries
      .iter()
      .map(|entry| parse_native_input_value("inner-witness", &entry.name, &entry.value))
      .collect::<Result<Vec<_>, _>>()?;

    let outer_statement = DirectOuterStatementInput {
      field_names: package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect(),
      public_inputs: package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| parse_native_input_value("outer-statement", &entry.name, &entry.value))
        .collect::<Result<Vec<_>, _>>()?,
    };

    if outer_statement.public_inputs != inner_verifier_public_inputs {
      return Err(OuterProofBackendError::UnsupportedStatementLayout {
        reason: "current arkworks outer lane only supports an outer statement that mirrors inner verifier public-input values exactly"
          .to_owned(),
      });
    }

    Ok(DirectOuterCircuitInput {
      source_artifact_id: package.witness.source_artifact_id.clone(),
      inner_proof,
      inner_verification_key,
      inner_verifier_public_inputs,
      outer_statement,
    })
  }

  /// Builds the canonical outer wrapper circuit from package plus raw artifacts.
  ///
  /// # Errors
  ///
  /// Returns an error if adaptation fails or the circuit-owned input is not
  /// ready for synthesis under the frozen outer statement contract.
  pub fn build_outer_circuit(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<OuterWrapperCircuit, OuterProofBackendError> {
    let adapted = self.adapt_input(package, artifacts)?;
    let circuit = OuterWrapperCircuit::from_input_for_host(
      adapted.to_circuit_input(),
      self.metadata().outer_host,
    );
    circuit.assert_ready_for_synthesis().map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid { reason: error.to_string() }
    })?;
    Ok(circuit)
  }
}
  };
}

impl_direct_backend_adaptation!(MidnightDirectOuterBackend);
impl_direct_backend_adaptation!(MidnightDirectOuterBackendBls12Host);
