use ff::Field;
use wrapper_core::WrapperError;

use super::OuterHostField;

/// Semantics currently supported by the outer wrapper statement.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OuterStatementSemantics {
  /// The outer statement mirrors the ordered inner verifier public inputs exactly.
  MirrorInnerPublicInputs,
}

/// Public statement exposed by the outer wrapper circuit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterStatementInput {
  /// Chosen statement semantics.
  pub semantics: OuterStatementSemantics,
  /// Ordered semantic field names.
  pub field_names: Vec<String>,
  /// Ordered public input values exposed by the outer circuit.
  pub public_inputs: Vec<OuterHostField>,
}

impl OuterStatementInput {
  /// Builds an outer statement input.
  #[must_use]
  pub fn new(
    semantics: OuterStatementSemantics,
    field_names: Vec<String>,
    public_inputs: Vec<OuterHostField>,
  ) -> Self {
    Self { semantics, field_names, public_inputs }
  }

  /// Returns a witness-free variant for Halo2's `without_witnesses` hook.
  #[must_use]
  pub fn without_witnesses(&self) -> Self {
    Self {
      semantics: self.semantics,
      field_names: self.field_names.clone(),
      public_inputs: vec![OuterHostField::ZERO; self.public_inputs.len()],
    }
  }

  /// Validates the outer statement against the current inner public-input contract.
  ///
  /// # Errors
  ///
  /// Returns an error if field names and values are inconsistent or if the
  /// mirror-statement contract is violated.
  pub fn validate_against_inner_inputs(
    &self,
    inner_public_inputs: &[OuterHostField],
  ) -> Result<(), WrapperError> {
    if self.field_names.len() != self.public_inputs.len() {
      return Err(WrapperError::InvalidInput {
        context: "outer statement",
        reason: format!(
          "field-name arity mismatch: expected {} names for {} values",
          self.public_inputs.len(),
          self.field_names.len()
        ),
      });
    }

    match self.semantics {
      OuterStatementSemantics::MirrorInnerPublicInputs => {
        if self.public_inputs.len() != inner_public_inputs.len() {
          return Err(WrapperError::InvalidInput {
            context: "outer statement",
            reason: format!(
              "mirror-statement arity mismatch: expected {}, got {}",
              inner_public_inputs.len(),
              self.public_inputs.len()
            ),
          });
        }

        if self.public_inputs != inner_public_inputs {
          return Err(WrapperError::InvalidInput {
            context: "outer statement",
            reason: "mirror statement values do not match inner verifier public inputs".to_owned(),
          });
        }
      }
    }

    Ok(())
  }
}
