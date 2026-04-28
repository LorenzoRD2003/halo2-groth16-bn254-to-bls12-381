use std::collections::{BTreeMap, BTreeSet};

use super::{
  CanonicalClassId, EqualityEdge, Halo2CellRef, Halo2R1csMetadata, R1csBuildError, VariableId,
};

/// Deterministic union-find over Halo2 cells.
///
/// The canonical representative of each class is always the minimum
/// `Halo2CellRef` in that class.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CanonicalCellUnionFind {
  parents: BTreeMap<Halo2CellRef, Halo2CellRef>,
}

impl CanonicalCellUnionFind {
  /// Creates an empty deterministic union-find.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Registers one cell in the union-find.
  pub fn add_cell(&mut self, cell: Halo2CellRef) {
    self.parents.entry(cell).or_insert(cell);
  }

  /// Adds one equality edge.
  ///
  /// # Errors
  ///
  /// Returns an error if the equality state becomes inconsistent.
  pub fn add_equality_edge(&mut self, edge: EqualityEdge) -> Result<(), R1csBuildError> {
    self.union(edge.left, edge.right)
  }

  /// Finds the canonical representative for one cell.
  ///
  /// # Errors
  ///
  /// Returns an error if the cell has not been registered.
  pub fn find_representative(
    &mut self,
    cell: Halo2CellRef,
  ) -> Result<Halo2CellRef, R1csBuildError> {
    let parent = *self.parents.get(&cell).ok_or(R1csBuildError::MissingCellAssignment { cell })?;
    if parent == cell {
      return Ok(cell);
    }

    let representative = self.find_representative(parent)?;
    self.parents.insert(cell, representative);
    Ok(representative)
  }

  fn union(&mut self, left: Halo2CellRef, right: Halo2CellRef) -> Result<(), R1csBuildError> {
    self.add_cell(left);
    self.add_cell(right);

    let left_representative = self.find_representative(left)?;
    let right_representative = self.find_representative(right)?;
    if left_representative == right_representative {
      return Ok(());
    }

    let canonical_representative = left_representative.min(right_representative);
    let non_canonical_representative = left_representative.max(right_representative);
    self.parents.insert(non_canonical_representative, canonical_representative);
    Ok(())
  }
}

/// Deterministic Halo2-cell to canonical-variable assignment map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Halo2CellAssignmentMap {
  /// Canonical equality class for each registered Halo2 cell.
  pub cell_to_class: BTreeMap<Halo2CellRef, CanonicalClassId>,
  /// Canonical R1CS variable chosen for each equality class.
  pub class_to_variable: BTreeMap<CanonicalClassId, VariableId>,
  /// Public-input variables in canonical order.
  pub public_variables: Vec<VariableId>,
}

impl Halo2CellAssignmentMap {
  /// Builds a canonical assignment map from cells plus equality edges.
  ///
  /// # Errors
  ///
  /// Returns an error if equality-class derivation becomes inconsistent.
  pub fn from_cells_and_edges<I, E>(cells: I, edges: E) -> Result<Self, R1csBuildError>
  where
    I: IntoIterator<Item = Halo2CellRef>,
    E: IntoIterator<Item = EqualityEdge>,
  {
    let mut all_cells: BTreeSet<Halo2CellRef> = cells.into_iter().collect();
    let normalized_edges: BTreeSet<EqualityEdge> =
      edges.into_iter().map(EqualityEdge::normalized).collect();

    for edge in &normalized_edges {
      all_cells.insert(edge.left);
      all_cells.insert(edge.right);
    }

    let mut equality = CanonicalCellUnionFind::new();
    for cell in &all_cells {
      equality.add_cell(*cell);
    }
    for edge in normalized_edges {
      equality.add_equality_edge(edge)?;
    }

    let mut cell_to_class = BTreeMap::new();
    for cell in all_cells {
      let representative = equality.find_representative(cell)?;
      cell_to_class.insert(cell, CanonicalClassId::new(representative));
    }

    let classes: BTreeSet<CanonicalClassId> = cell_to_class.values().copied().collect();
    let mut class_to_variable = BTreeMap::new();
    let mut public_variables = Vec::new();
    for (index, class_id) in classes.into_iter().enumerate() {
      let variable = VariableId(index as u32);
      if class_to_variable.insert(class_id, variable).is_some() {
        return Err(R1csBuildError::InconsistentEquality);
      }
      if class_id.is_public() {
        public_variables.push(variable);
      }
    }

    let map = Self { cell_to_class, class_to_variable, public_variables };
    let _ = map.variable_partition()?;
    Ok(map)
  }

  /// Builds a canonical assignment map from validated Halo2 metadata.
  ///
  /// # Errors
  ///
  /// Returns an error if metadata validation fails or if distinct public-input
  /// slots resolve to the same canonical variable.
  pub fn from_metadata(metadata: &Halo2R1csMetadata) -> Result<Self, R1csBuildError> {
    metadata.validate()?;

    let canonical_cells = metadata.canonical_cells();
    let normalized_edges: BTreeSet<EqualityEdge> =
      metadata.equality_edges.iter().copied().map(EqualityEdge::normalized).collect();

    let mut equality = CanonicalCellUnionFind::new();
    for cell in &canonical_cells {
      equality.add_cell(*cell);
    }
    for edge in &normalized_edges {
      equality.add_equality_edge(*edge)?;
    }

    let mut cell_to_class = BTreeMap::new();
    for cell in canonical_cells {
      let representative = equality.find_representative(cell)?;
      cell_to_class.insert(cell, CanonicalClassId::new(representative));
    }

    let classes: BTreeSet<CanonicalClassId> = cell_to_class.values().copied().collect();
    let mut class_to_variable = BTreeMap::new();
    for (index, class_id) in classes.into_iter().enumerate() {
      let previous = class_to_variable.insert(class_id, VariableId(index as u32));
      if previous.is_some() {
        return Err(R1csBuildError::InconsistentEquality);
      }
    }

    let public_input_by_index = metadata
      .public_inputs
      .iter()
      .map(|public_input| (public_input.public_index, public_input.cell))
      .collect::<BTreeMap<_, _>>();

    let mut seen_public_variables = BTreeSet::new();
    let mut public_variables = Vec::with_capacity(public_input_by_index.len());
    for cell in public_input_by_index.values().copied() {
      let class_id = cell_to_class.get(&cell).copied().ok_or(R1csBuildError::UnknownCell(cell))?;
      let variable =
        class_to_variable.get(&class_id).copied().ok_or(R1csBuildError::InconsistentEquality)?;
      if !seen_public_variables.insert(variable) {
        return Err(R1csBuildError::DuplicatePublicInputVariable(variable));
      }
      public_variables.push(variable);
    }

    let map = Self { cell_to_class, class_to_variable, public_variables };
    let _ = map.variable_partition()?;
    Ok(map)
  }

  /// Returns the canonical class for one cell.
  ///
  /// # Errors
  ///
  /// Returns an error if the cell is not present in the assignment map.
  pub fn class_for(&self, cell: Halo2CellRef) -> Result<CanonicalClassId, R1csBuildError> {
    self.cell_to_class.get(&cell).copied().ok_or(R1csBuildError::MissingCellAssignment { cell })
  }

  /// Returns the canonical variable for one cell.
  ///
  /// # Errors
  ///
  /// Returns an error if the cell is not present or the class-to-variable
  /// mapping became inconsistent.
  pub fn variable_for(&self, cell: Halo2CellRef) -> Result<VariableId, R1csBuildError> {
    let class_id = self.class_for(cell)?;
    self.class_to_variable.get(&class_id).copied().ok_or(R1csBuildError::InconsistentEquality)
  }

  /// Returns the public-input variables in frontend-provided public-input order.
  #[must_use]
  pub fn public_variables(&self) -> &[VariableId] {
    &self.public_variables
  }

  /// Returns the witness-variable partition in canonical order.
  ///
  /// # Errors
  ///
  /// Returns an error if the stored public-variable partition is inconsistent
  /// with the canonical class ordering.
  pub fn witness_variables(&self) -> Result<Vec<VariableId>, R1csBuildError> {
    let (public_inputs, witnesses) = self.variable_partition()?;
    if public_inputs != self.public_variables {
      return Err(R1csBuildError::InconsistentEquality);
    }
    Ok(witnesses)
  }

  /// Returns `(public_inputs, witnesses)` in canonical variable order.
  ///
  /// # Errors
  ///
  /// Returns an error if the stored public-variable partition is inconsistent
  /// with the canonical class ordering.
  pub fn variable_partition(&self) -> Result<(Vec<VariableId>, Vec<VariableId>), R1csBuildError> {
    let public_input_set = self.public_variables.iter().copied().collect::<BTreeSet<_>>();
    let public_inputs = self.public_variables.clone();
    let witnesses = self
      .class_to_variable
      .values()
      .filter_map(|variable| (!public_input_set.contains(variable)).then_some(*variable))
      .collect::<Vec<_>>();

    Ok((public_inputs, witnesses))
  }
}
