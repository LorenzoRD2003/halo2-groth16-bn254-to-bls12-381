use thiserror::Error;

use super::{Halo2CellRef, VariableId};

/// Errors raised while building canonical R1CS.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum R1csBuildError {
  /// Halo2 equality classes or their derived ordering are inconsistent.
  #[error("canonical R1CS equality classes are inconsistent")]
  InconsistentEquality,
  /// A Halo2 cell could not be mapped into the canonical assignment map.
  #[error("canonical R1CS is missing a Halo2 cell assignment for {cell:?}")]
  MissingCellAssignment {
    /// Halo2 cell that was missing from the canonical assignment map.
    cell: Halo2CellRef,
  },
  /// A public input referred to a non-instance cell.
  #[error("canonical R1CS public input must reference an instance cell, got {0:?}")]
  InvalidPublicInputCell(Halo2CellRef),
  /// More than one public input used the same public index.
  #[error("canonical R1CS public input index {0} is duplicated")]
  DuplicatePublicInputIndex(usize),
  /// Public input indices were not the contiguous range `0..n`.
  #[error("canonical R1CS public input indices must be contiguous from 0..n")]
  NonContiguousPublicInputIndices,
  /// Metadata referenced a cell that was not declared in the canonical cell set.
  #[error("canonical R1CS metadata referenced unknown cell {0:?}")]
  UnknownCell(Halo2CellRef),
  /// Two public input references resolved to the same canonical variable.
  #[error("canonical R1CS public input variables must be distinct, got duplicate {0:?}")]
  DuplicatePublicInputVariable(VariableId),
  /// A zkInterface bridge export diverged from the canonical R1CS circuit.
  #[error("zkInterface bridge export mismatch in {context}")]
  ZkInterfaceExportMismatch {
    /// Export field that no longer matches the canonical circuit.
    context: &'static str,
  },
  /// A required public assignment was not provided.
  #[error("canonical R1CS is missing public assignment for {0:?}")]
  MissingPublicAssignment(VariableId),
  /// A required witness assignment was not provided.
  #[error("canonical R1CS is missing witness assignment for {0:?}")]
  MissingWitnessAssignment(VariableId),
  /// A public variable was incorrectly provided as a private witness.
  #[error("canonical R1CS public variable was provided as a witness {0:?}")]
  PublicVariablePassedAsWitness(VariableId),
  /// An unexpected public assignment key was provided.
  #[error("canonical R1CS received unexpected public assignment for {0:?}")]
  UnexpectedPublicAssignment(VariableId),
  /// An unexpected witness assignment key was provided.
  #[error("canonical R1CS received unexpected witness assignment for {0:?}")]
  UnexpectedWitnessAssignment(VariableId),
  /// Arkworks rejected constraint synthesis for this R1CS adapter.
  #[error("arkworks synthesis error: {0}")]
  ArkworksSynthesisError(String),
  /// Arkworks rejected setup/proof/verification for this R1CS adapter.
  #[error("arkworks proof error: {0}")]
  ArkworksProofError(String),
  /// A constraint referenced a variable that has not been allocated.
  #[error("canonical R1CS references undeclared variable {var:?}")]
  UndeclaredVariable {
    /// Undeclared variable referenced by the constraint being added.
    var: VariableId,
  },
}
