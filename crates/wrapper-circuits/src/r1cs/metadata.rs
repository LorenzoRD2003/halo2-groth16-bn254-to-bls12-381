use std::collections::{BTreeMap, BTreeSet};

use super::{EqualityEdge, Halo2CellRef, R1csBuildError};

/// One frontend-provided public-input binding for canonical lowering.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Halo2PublicInputRef {
  /// Halo2 cell exposed as a public input.
  pub cell: Halo2CellRef,
  /// Frontend-defined public input position.
  pub public_index: usize,
}

/// Explicit metadata boundary for Phase 3 Halo2-to-R1CS lowering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Halo2R1csMetadata {
  /// Cells that participate in the currently supported lowering subset.
  pub cells: Vec<Halo2CellRef>,
  /// Equality/copy edges over those cells.
  pub equality_edges: Vec<EqualityEdge>,
  /// Public-input bindings in frontend order.
  pub public_inputs: Vec<Halo2PublicInputRef>,
}

impl Halo2R1csMetadata {
  /// Returns the canonical cell set after deterministic deduplication.
  #[must_use]
  pub fn canonical_cells(&self) -> BTreeSet<Halo2CellRef> {
    self.cells.iter().copied().collect()
  }

  /// Validates the metadata boundary before lowering.
  ///
  /// # Errors
  ///
  /// Returns an error if public-input or equality metadata references unknown
  /// cells or violates the deterministic public-input contract.
  pub fn validate(&self) -> Result<(), R1csBuildError> {
    let canonical_cells = self.canonical_cells();
    let mut public_input_indices = BTreeSet::new();
    let mut public_input_by_index = BTreeMap::new();

    for public_input in &self.public_inputs {
      if !matches!(public_input.cell, Halo2CellRef::Instance { .. }) {
        return Err(R1csBuildError::InvalidPublicInputCell(public_input.cell));
      }
      if !canonical_cells.contains(&public_input.cell) {
        return Err(R1csBuildError::UnknownCell(public_input.cell));
      }
      if !public_input_indices.insert(public_input.public_index) {
        return Err(R1csBuildError::DuplicatePublicInputIndex(public_input.public_index));
      }
      public_input_by_index.insert(public_input.public_index, public_input.cell);
    }

    if public_input_by_index
      .keys()
      .copied()
      .enumerate()
      .any(|(expected_index, actual_index)| expected_index != actual_index)
    {
      return Err(R1csBuildError::NonContiguousPublicInputIndices);
    }

    for edge in &self.equality_edges {
      if !canonical_cells.contains(&edge.left) {
        return Err(R1csBuildError::UnknownCell(edge.left));
      }
      if !canonical_cells.contains(&edge.right) {
        return Err(R1csBuildError::UnknownCell(edge.right));
      }
    }

    Ok(())
  }
}
