use core::fmt;

use blake2::{
  Blake2bVar,
  digest::{Update, VariableOutput},
};
use ff::PrimeField;

use super::{LinearCombination, LinearTerm, R1csCircuit, R1csConstraint, VariableId};
use crate::NativeField;

/// Domain separator for the canonical Phase 4 R1CS identity encoding.
pub const R1CS_IDENTITY_DOMAIN_SEPARATOR: &[u8] = b"halo2-groth16-wrapper:r1cs:v1";

/// Stable 32-byte hash of the canonical R1CS encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct R1csIdentityHash(pub [u8; 32]);

impl fmt::Display for R1csIdentityHash {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in self.0 {
      write!(f, "{byte:02x}")?;
    }
    Ok(())
  }
}

impl R1csCircuit {
  /// Returns the canonical byte encoding for this R1CS circuit.
  #[must_use]
  pub fn canonical_bytes(&self) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(R1CS_IDENTITY_DOMAIN_SEPARATOR);
    encode_u64(self.variable_count() as u64, &mut bytes);
    encode_u64(self.public_inputs.len() as u64, &mut bytes);
    for variable in &self.public_inputs {
      encode_variable_id(*variable, &mut bytes);
    }
    encode_u64(self.constraints.len() as u64, &mut bytes);
    for constraint in &self.constraints {
      encode_constraint(constraint, &mut bytes);
    }
    bytes
  }

  /// Returns the stable identity hash for this canonical R1CS circuit.
  #[must_use]
  pub fn identity_hash(&self) -> R1csIdentityHash {
    let canonical_bytes = self.canonical_bytes();
    let mut hasher = Blake2bVar::new(32).expect("32-byte BLAKE2b output size should be valid");
    hasher.update(&canonical_bytes);

    let mut digest = [0_u8; 32];
    hasher
      .finalize_variable(&mut digest)
      .expect("32-byte output buffer should match the configured BLAKE2b output size");
    R1csIdentityHash(digest)
  }
}

fn encode_constraint(constraint: &R1csConstraint, bytes: &mut Vec<u8>) {
  encode_linear_combination(&constraint.a, bytes);
  encode_linear_combination(&constraint.b, bytes);
  encode_linear_combination(&constraint.c, bytes);
}

fn encode_linear_combination(linear_combination: &LinearCombination, bytes: &mut Vec<u8>) {
  let normalized =
    LinearCombination::new(linear_combination.terms.clone(), linear_combination.constant);
  encode_u64(normalized.terms.len() as u64, bytes);
  for term in &normalized.terms {
    encode_linear_term(term, bytes);
  }
  encode_field_element(normalized.constant, bytes);
}

fn encode_linear_term(term: &LinearTerm, bytes: &mut Vec<u8>) {
  encode_variable_id(term.var, bytes);
  encode_field_element(term.coeff, bytes);
}

fn encode_variable_id(variable_id: VariableId, bytes: &mut Vec<u8>) {
  encode_u64(u64::from(variable_id.index()), bytes);
}

fn encode_field_element(field_element: NativeField, bytes: &mut Vec<u8>) {
  bytes.extend_from_slice(field_element.to_repr().as_ref());
}

fn encode_u64(value: u64, bytes: &mut Vec<u8>) {
  bytes.extend_from_slice(&value.to_le_bytes());
}
