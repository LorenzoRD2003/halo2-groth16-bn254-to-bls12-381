use ff::Field;
use wrapper_core::WrapperError;

use crate::{
  Bls12HostField, ForeignField, OUTER_VK_COMMITMENT_FIELD_NAME,
  groth16_vk_commitment_bls12_public_input_names, groth16_vk_commitment_bls12_public_inputs,
  groth16_vk_commitment_public_input_names, groth16_vk_commitment_public_inputs,
};

use super::OuterHostField;

/// Semantics currently supported by the outer wrapper statement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OuterStatementSemantics {
  /// The outer statement mirrors inner public inputs and binds one VK commitment.
  MirrorInnerPublicInputsAndVerificationKeyCommitment,
}

/// Explicit public commitment to the witness-side verification key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterVerificationKeyCommitment {
  /// Semantic field name for the commitment.
  pub field_name: String,
  /// Canonical semantic commitment value.
  pub value: OuterVerificationKeyCommitmentValue,
}

/// Semantic VK commitment value carried by the outer statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OuterVerificationKeyCommitmentValue {
  /// BN254-compat semantic commitment.
  Bn254(ForeignField),
  /// BLS12 host-native semantic commitment.
  Bls12(Bls12HostField),
}

impl OuterVerificationKeyCommitment {
  /// Builds one explicit verification-key commitment component.
  #[must_use]
  pub fn new(
    field_name: impl Into<String>,
    value: OuterVerificationKeyCommitmentValue,
  ) -> Self {
    Self { field_name: field_name.into(), value }
  }

  /// Returns the flattened field names used by the current host-lane exposure.
  #[must_use]
  pub fn flattened_field_names(&self) -> Vec<String> {
    match self.value {
      OuterVerificationKeyCommitmentValue::Bn254(_) => {
        groth16_vk_commitment_public_input_names(&self.field_name)
      }
      OuterVerificationKeyCommitmentValue::Bls12(_) => {
        groth16_vk_commitment_bls12_public_input_names(&self.field_name)
      }
    }
  }

  /// Returns the flattened host-field values used by the current host-lane exposure.
  #[must_use]
  pub fn flattened_public_inputs(&self) -> Vec<OuterHostField> {
    match self.value {
      OuterVerificationKeyCommitmentValue::Bn254(value) => groth16_vk_commitment_public_inputs(value),
      OuterVerificationKeyCommitmentValue::Bls12(value) => {
        groth16_vk_commitment_bls12_public_inputs(value)
      }
    }
  }
}

/// Public statement exposed by the outer wrapper circuit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterStatementInput {
  /// Chosen statement semantics.
  pub semantics: OuterStatementSemantics,
  /// Ordered mirrored inner public-input names.
  pub mirrored_field_names: Vec<String>,
  /// Ordered mirrored inner public-input values.
  pub mirrored_public_inputs: Vec<OuterHostField>,
  /// Explicit public verification-key commitment.
  pub vk_commitment: OuterVerificationKeyCommitment,
  /// Ordered flat semantic field names exposed by the outer circuit.
  pub field_names: Vec<String>,
  /// Ordered flat public input values exposed by the outer circuit.
  pub public_inputs: Vec<OuterHostField>,
}

impl OuterStatementInput {
  /// Builds an explicit outer statement input.
  #[must_use]
  pub fn new(
    semantics: OuterStatementSemantics,
    mirrored_field_names: Vec<String>,
    mirrored_public_inputs: Vec<OuterHostField>,
    vk_commitment: OuterVerificationKeyCommitment,
  ) -> Self {
    let mut field_names = mirrored_field_names.clone();
    field_names.extend(vk_commitment.flattened_field_names());

    let mut public_inputs = mirrored_public_inputs.clone();
    public_inputs.extend(vk_commitment.flattened_public_inputs());

    Self {
      semantics,
      mirrored_field_names,
      mirrored_public_inputs,
      vk_commitment,
      field_names,
      public_inputs,
    }
  }

  /// Returns a witness-free variant for Halo2's `without_witnesses` hook.
  #[must_use]
  pub fn without_witnesses(&self) -> Self {
    let zero_value = match self.vk_commitment.value {
      OuterVerificationKeyCommitmentValue::Bn254(_) => {
        OuterVerificationKeyCommitmentValue::Bn254(ForeignField::ZERO)
      }
      OuterVerificationKeyCommitmentValue::Bls12(_) => {
        OuterVerificationKeyCommitmentValue::Bls12(Bls12HostField::ZERO)
      }
    };
    Self::new(
      self.semantics,
      self.mirrored_field_names.clone(),
      vec![OuterHostField::ZERO; self.mirrored_public_inputs.len()],
      OuterVerificationKeyCommitment::new(self.vk_commitment.field_name.clone(), zero_value),
    )
  }

  /// Validates the outer statement against the current inner public-input contract.
  ///
  /// # Errors
  ///
  /// Returns an error if field names and values are inconsistent or if the
  /// explicit mirror-plus-commitment contract is violated.
  pub fn validate_against_inner_inputs_and_vk(
    &self,
    inner_public_inputs: &[OuterHostField],
    expected_vk_commitment: &OuterVerificationKeyCommitmentValue,
  ) -> Result<(), WrapperError> {
    if self.mirrored_field_names.len() != self.mirrored_public_inputs.len() {
      return Err(WrapperError::InvalidInput {
        context: "outer statement",
        reason: format!(
          "mirrored field-name arity mismatch: expected {} names for {} values",
          self.mirrored_public_inputs.len(),
          self.mirrored_field_names.len()
        ),
      });
    }

    if self.field_names.len() != self.public_inputs.len() {
      return Err(WrapperError::InvalidInput {
        context: "outer statement",
        reason: format!(
          "flattened field-name arity mismatch: expected {} names for {} values",
          self.public_inputs.len(),
          self.field_names.len()
        ),
      });
    }

    match self.semantics {
      OuterStatementSemantics::MirrorInnerPublicInputsAndVerificationKeyCommitment => {
        if self.mirrored_public_inputs.len() != inner_public_inputs.len() {
          return Err(WrapperError::InvalidInput {
            context: "outer statement",
            reason: format!(
              "mirror-statement arity mismatch: expected {}, got {}",
              inner_public_inputs.len(),
              self.mirrored_public_inputs.len()
            ),
          });
        }

        if self.mirrored_public_inputs != inner_public_inputs {
          return Err(WrapperError::InvalidInput {
            context: "outer statement",
            reason: "mirror statement values do not match inner verifier public inputs".to_owned(),
          });
        }

        if self.vk_commitment.field_name != OUTER_VK_COMMITMENT_FIELD_NAME {
          return Err(WrapperError::InvalidInput {
            context: "outer statement",
            reason: format!(
              "unexpected VK commitment field name: expected '{OUTER_VK_COMMITMENT_FIELD_NAME}', got '{}'",
              self.vk_commitment.field_name
            ),
          });
        }

        if &self.vk_commitment.value != expected_vk_commitment {
          return Err(WrapperError::InvalidInput {
            context: "outer statement",
            reason: "VK commitment value does not match the inner verification key".to_owned(),
          });
        }

        let mut expected_field_names = self.mirrored_field_names.clone();
        expected_field_names.extend(self.vk_commitment.flattened_field_names());
        if self.field_names != expected_field_names {
          return Err(WrapperError::InvalidInput {
            context: "outer statement",
            reason: "flattened outer statement field order does not match the explicit statement components"
              .to_owned(),
          });
        }

        let mut expected_public_inputs = self.mirrored_public_inputs.clone();
        expected_public_inputs.extend(self.vk_commitment.flattened_public_inputs());
        if self.public_inputs != expected_public_inputs {
          return Err(WrapperError::InvalidInput {
            context: "outer statement",
            reason:
              "flattened outer statement values do not match the explicit statement components"
                .to_owned(),
          });
        }
      }
    }

    Ok(())
  }
}
