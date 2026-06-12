use ff::Field;
use wrapper_core::WrapperError;

use crate::{
  Groth16Bn254Proof, Groth16Bn254VerifyingKey, OUTER_VK_COMMITMENT_FIELD_NAME,
  OuterHostFlavor, groth16_vk_commitment, groth16_vk_commitment_bls12,
};

use super::{
  OuterHostField, OuterStatementInput, OuterStatementSemantics, OuterVerificationKeyCommitment,
  OuterVerificationKeyCommitmentValue,
};

/// Canonical input for the outer wrapper circuit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterWrapperCircuitInput {
  /// Inner Groth16 BN254 proof material.
  pub inner_proof: Groth16Bn254Proof,
  /// Inner Groth16 BN254 verification key material.
  pub inner_verification_key: Groth16Bn254VerifyingKey,
  /// Ordered inner verifier public inputs.
  pub inner_public_inputs: Vec<OuterHostField>,
  /// Public outer statement exposed by the circuit.
  pub outer_statement: OuterStatementInput,
}

impl OuterWrapperCircuitInput {
  /// Builds a canonical outer wrapper circuit input.
  #[must_use]
  pub fn new(
    inner_proof: Groth16Bn254Proof,
    inner_verification_key: Groth16Bn254VerifyingKey,
    inner_public_inputs: Vec<OuterHostField>,
    outer_statement: OuterStatementInput,
  ) -> Self {
    Self { inner_proof, inner_verification_key, inner_public_inputs, outer_statement }
  }

  /// Builds an explicit mirror-plus-commitment outer wrapper input.
  #[must_use]
  pub fn explicit(
    inner_proof: Groth16Bn254Proof,
    inner_verification_key: Groth16Bn254VerifyingKey,
    inner_public_inputs: Vec<OuterHostField>,
    mirrored_field_names: Vec<String>,
  ) -> Self {
    Self::explicit_for_outer_host(
      inner_proof,
      inner_verification_key,
      inner_public_inputs,
      mirrored_field_names,
      OuterHostFlavor::MidnightBn254,
    )
  }

  /// Builds an explicit mirror-plus-commitment outer wrapper input for one chosen outer host lane.
  #[must_use]
  pub fn explicit_for_outer_host(
    inner_proof: Groth16Bn254Proof,
    inner_verification_key: Groth16Bn254VerifyingKey,
    inner_public_inputs: Vec<OuterHostField>,
    mirrored_field_names: Vec<String>,
    outer_host: OuterHostFlavor,
  ) -> Self {
    let vk_commitment = match outer_host {
      OuterHostFlavor::MidnightBn254 => {
        OuterVerificationKeyCommitmentValue::Bn254(groth16_vk_commitment(&inner_verification_key))
      }
      OuterHostFlavor::MidnightBls12_381 => {
        OuterVerificationKeyCommitmentValue::Bls12(groth16_vk_commitment_bls12(&inner_verification_key))
      }
    };
    let outer_statement = OuterStatementInput::new(
      OuterStatementSemantics::MirrorInnerPublicInputsAndVerificationKeyCommitment,
      mirrored_field_names,
      inner_public_inputs.clone(),
      OuterVerificationKeyCommitment::new(OUTER_VK_COMMITMENT_FIELD_NAME, vk_commitment),
    );
    Self::new(inner_proof, inner_verification_key, inner_public_inputs, outer_statement)
  }

  /// Returns a witness-free variant for Halo2's `without_witnesses` hook.
  #[must_use]
  pub fn without_witnesses(&self) -> Self {
    Self {
      inner_proof: self.inner_proof.clone(),
      inner_verification_key: self.inner_verification_key.clone(),
      inner_public_inputs: vec![OuterHostField::ZERO; self.inner_public_inputs.len()],
      outer_statement: self.outer_statement.without_witnesses(),
    }
  }

  /// Validates that the circuit input satisfies the frozen outer wrapper contract.
  ///
  /// # Errors
  ///
  /// Returns an error if the inner verification key arity is inconsistent or
  /// the outer statement does not match the ordered inner public inputs and VK commitment.
  pub fn validate_for_outer_host(&self, outer_host: OuterHostFlavor) -> Result<(), WrapperError> {
    let expected_ic_len = self.inner_public_inputs.len() + 1;
    if self.inner_verification_key.ic.len() != expected_ic_len {
      return Err(WrapperError::InvalidInput {
        context: "outer wrapper circuit input",
        reason: format!(
          "inner verification key IC length mismatch: expected {expected_ic_len}, got {}",
          self.inner_verification_key.ic.len()
        ),
      });
    }

    let expected_vk_commitment = match outer_host {
      OuterHostFlavor::MidnightBn254 => {
        OuterVerificationKeyCommitmentValue::Bn254(groth16_vk_commitment(&self.inner_verification_key))
      }
      OuterHostFlavor::MidnightBls12_381 => OuterVerificationKeyCommitmentValue::Bls12(
        groth16_vk_commitment_bls12(&self.inner_verification_key),
      ),
    };

    self
      .outer_statement
      .validate_against_inner_inputs_and_vk(&self.inner_public_inputs, &expected_vk_commitment)
  }
}
