use std::collections::BTreeMap;

use ff::PrimeField;

use super::{
  LinearCombination, R1csBuildError, R1csCircuit, R1csConstraint, R1csIdentityHash, VariableId,
};
use crate::NativeField;

/// Internal zkInterface-style bridge export for canonical R1CS.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZkInterfaceR1csExport {
  /// Field modulus in canonical little-endian byte order.
  pub field_modulus_le: Vec<u8>,
  /// Total number of non-constant variables in the circuit.
  pub num_variables: usize,
  /// Public variables in canonical public-input order.
  pub public_variables: Vec<VariableId>,
  /// Constraints in canonical R1CS order.
  pub constraints: Vec<ZkInterfaceConstraint>,
  /// Canonical identity hash of the exported circuit.
  pub identity_hash: R1csIdentityHash,
}

/// Internal zkInterface-style R1CS constraint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZkInterfaceConstraint {
  /// Left multiplicand.
  pub a: ZkInterfaceLinearCombination,
  /// Right multiplicand.
  pub b: ZkInterfaceLinearCombination,
  /// Output linear combination.
  pub c: ZkInterfaceLinearCombination,
}

/// Internal zkInterface-style sparse linear combination.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZkInterfaceLinearCombination {
  /// Sparse terms in canonical order.
  pub terms: Vec<ZkInterfaceTerm>,
  /// Explicit constant term.
  pub constant: NativeField,
}

/// Internal zkInterface-style sparse term.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZkInterfaceTerm {
  /// Variable referenced by this term.
  pub variable: VariableId,
  /// Coefficient on that variable.
  pub coefficient: NativeField,
}

/// Deterministic witness export for the zkInterface bridge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZkInterfaceWitnessExport {
  /// Assignments sorted by `VariableId`.
  pub assignments: Vec<ZkInterfaceWitnessAssignment>,
}

/// One deterministic witness assignment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ZkInterfaceWitnessAssignment {
  /// Assigned variable id.
  pub variable: VariableId,
  /// Assigned field value.
  pub value: NativeField,
}

impl R1csCircuit {
  /// Returns the public variables in canonical public-input order.
  #[must_use]
  pub fn public_variables(&self) -> &[VariableId] {
    &self.public_inputs
  }

  /// Exports the canonical R1CS into the internal zkInterface bridge model.
  #[must_use]
  pub fn to_zkinterface_export(&self) -> ZkInterfaceR1csExport {
    ZkInterfaceR1csExport {
      field_modulus_le: field_modulus_le_bytes(),
      num_variables: self.variable_count(),
      public_variables: self.public_inputs.clone(),
      constraints: self.constraints.iter().map(export_constraint).collect(),
      identity_hash: self.identity_hash(),
    }
  }
}

impl ZkInterfaceR1csExport {
  /// Validates this export against the canonical R1CS circuit it was derived from.
  ///
  /// # Errors
  ///
  /// Returns an error if any exported structural field differs from the
  /// canonical circuit view.
  pub fn validate_against_circuit(&self, circuit: &R1csCircuit) -> Result<(), R1csBuildError> {
    let expected = circuit.to_zkinterface_export();

    if self.identity_hash != expected.identity_hash {
      return Err(R1csBuildError::ZkInterfaceExportMismatch { context: "identity hash" });
    }
    if self.field_modulus_le != expected.field_modulus_le {
      return Err(R1csBuildError::ZkInterfaceExportMismatch { context: "field modulus" });
    }
    if self.num_variables != expected.num_variables {
      return Err(R1csBuildError::ZkInterfaceExportMismatch { context: "variable count" });
    }
    if self.public_variables != expected.public_variables {
      return Err(R1csBuildError::ZkInterfaceExportMismatch { context: "public variables" });
    }
    if self.constraints != expected.constraints {
      return Err(R1csBuildError::ZkInterfaceExportMismatch { context: "constraints" });
    }

    Ok(())
  }
}

/// Exports witness assignments in deterministic variable order.
#[must_use]
pub fn export_witness(assignments: &BTreeMap<VariableId, NativeField>) -> ZkInterfaceWitnessExport {
  ZkInterfaceWitnessExport {
    assignments: assignments
      .iter()
      .map(|(variable, value)| ZkInterfaceWitnessAssignment { variable: *variable, value: *value })
      .collect(),
  }
}

fn export_constraint(constraint: &R1csConstraint) -> ZkInterfaceConstraint {
  ZkInterfaceConstraint {
    a: export_linear_combination(&constraint.a),
    b: export_linear_combination(&constraint.b),
    c: export_linear_combination(&constraint.c),
  }
}

fn export_linear_combination(
  linear_combination: &LinearCombination,
) -> ZkInterfaceLinearCombination {
  let normalized =
    LinearCombination::new(linear_combination.terms.clone(), linear_combination.constant);
  ZkInterfaceLinearCombination {
    terms: normalized
      .terms
      .into_iter()
      .map(|term| ZkInterfaceTerm { variable: term.var, coefficient: term.coeff })
      .collect(),
    constant: normalized.constant,
  }
}

fn field_modulus_le_bytes() -> Vec<u8> {
  let width = <NativeField as PrimeField>::Repr::default().as_ref().len();
  let mut bytes = vec![0_u8; width];

  if let Some(hex_modulus) = NativeField::MODULUS.strip_prefix("0x") {
    for ch in hex_modulus.bytes() {
      let nibble = match ch {
        b'0'..=b'9' => ch - b'0',
        b'a'..=b'f' => 10 + (ch - b'a'),
        b'A'..=b'F' => 10 + (ch - b'A'),
        _ => panic!("PrimeField::MODULUS should be ASCII hex or decimal"),
      };

      let mut carry = u16::from(nibble);
      for byte in &mut bytes {
        let value = (u16::from(*byte) * 16) + carry;
        *byte = (value & 0xff) as u8;
        carry = value >> 8;
      }
      debug_assert_eq!(carry, 0, "field modulus should fit in the field repr width");
    }
  } else {
    for ch in NativeField::MODULUS.bytes() {
      let digit = ch.wrapping_sub(b'0');
      debug_assert!(digit <= 9, "PrimeField::MODULUS should be an ASCII decimal string");

      let mut carry = u16::from(digit);
      for byte in &mut bytes {
        let value = (u16::from(*byte) * 10) + carry;
        *byte = (value & 0xff) as u8;
        carry = value >> 8;
      }
      debug_assert_eq!(carry, 0, "field modulus should fit in the field repr width");
    }
  }

  bytes
}
