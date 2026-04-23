use super::{
  CanonicalR1csBuilder, EqualityEdge, Halo2CellAssignmentMap, Halo2CellLinearCombination,
  Halo2CellRef, Halo2R1csMetadata, LinearCombination, LinearTerm, R1csBuildError, R1csCircuit,
  VariableId,
};
use crate::NativeField;

/// Narrow deterministic lowering boundary from Halo2-style cells to canonical R1CS.
#[derive(Clone, Debug)]
pub struct Halo2Phase1R1csLowering {
  assignment_map: Halo2CellAssignmentMap,
  builder: CanonicalR1csBuilder,
}

impl Halo2Phase1R1csLowering {
  /// Creates a lowering boundary from a precomputed canonical assignment map.
  ///
  /// # Errors
  ///
  /// Returns an error if the assignment map does not define a canonical
  /// variable partition.
  pub fn new(assignment_map: Halo2CellAssignmentMap) -> Result<Self, R1csBuildError> {
    let (public_inputs, witnesses) = assignment_map.variable_partition()?;
    let builder = CanonicalR1csBuilder::from_variable_partition(public_inputs, witnesses)?;
    Ok(Self { assignment_map, builder })
  }

  /// Creates a lowering boundary directly from cells plus equality edges.
  ///
  /// # Errors
  ///
  /// Returns an error if equality-class derivation or variable partitioning is
  /// inconsistent.
  pub fn from_cells_and_edges<I, E>(cells: I, edges: E) -> Result<Self, R1csBuildError>
  where
    I: IntoIterator<Item = Halo2CellRef>,
    E: IntoIterator<Item = EqualityEdge>,
  {
    Self::new(Halo2CellAssignmentMap::from_cells_and_edges(cells, edges)?)
  }

  /// Creates a lowering boundary directly from explicit Phase 3 metadata.
  ///
  /// # Errors
  ///
  /// Returns an error if metadata validation, equality derivation, or variable
  /// partitioning is inconsistent.
  pub fn from_metadata(metadata: &Halo2R1csMetadata) -> Result<Self, R1csBuildError> {
    Self::new(Halo2CellAssignmentMap::from_metadata(metadata)?)
  }

  /// Returns the canonical assignment map used by this lowering.
  #[must_use]
  pub fn assignment_map(&self) -> &Halo2CellAssignmentMap {
    &self.assignment_map
  }

  /// Returns the canonical variable assigned to one Halo2 cell.
  ///
  /// # Errors
  ///
  /// Returns an error if the cell is not present in the canonical assignment
  /// map.
  pub fn variable_for(&self, cell: Halo2CellRef) -> Result<VariableId, R1csBuildError> {
    self.assignment_map.variable_for(cell)
  }

  /// Returns the canonical variable assigned to one Halo2 cell.
  ///
  /// # Errors
  ///
  /// Returns an error if the cell is not present in the canonical assignment
  /// map.
  pub fn variable_for_cell(&self, cell: Halo2CellRef) -> Result<VariableId, R1csBuildError> {
    self.variable_for(cell)
  }

  /// Returns the public-input variables in frontend-provided public-input order.
  #[must_use]
  pub fn public_variables(&self) -> &[VariableId] {
    self.assignment_map.public_variables()
  }

  /// Records one already-supported algebraic R1CS constraint.
  ///
  /// # Errors
  ///
  /// Returns an error if any referenced cell is missing from the canonical
  /// assignment map.
  pub fn add_algebraic_constraint(
    &mut self,
    a: &Halo2CellLinearCombination,
    b: &Halo2CellLinearCombination,
    c: &Halo2CellLinearCombination,
  ) -> Result<(), R1csBuildError> {
    self.builder.add_constraint(
      self.lower_linear_combination(a)?,
      self.lower_linear_combination(b)?,
      self.lower_linear_combination(c)?,
    )
  }

  /// Records `lhs * rhs = output`.
  ///
  /// # Errors
  ///
  /// Returns an error if any referenced cell is missing from the canonical
  /// assignment map.
  pub fn add_multiplication_gate(
    &mut self,
    lhs: Halo2CellRef,
    rhs: Halo2CellRef,
    output: Halo2CellRef,
  ) -> Result<(), R1csBuildError> {
    self.builder.add_multiplication_constraint(
      self.variable_for(lhs)?,
      self.variable_for(rhs)?,
      self.variable_for(output)?,
    )
  }

  /// Records `var * constant = output`.
  ///
  /// # Errors
  ///
  /// Returns an error if any referenced cell is missing from the canonical
  /// assignment map.
  pub fn add_scaled_multiplication_gate(
    &mut self,
    var: Halo2CellRef,
    constant: NativeField,
    output: Halo2CellRef,
  ) -> Result<(), R1csBuildError> {
    self.builder.add_scaled_multiplication_constraint(
      self.variable_for(var)?,
      constant,
      self.variable_for(output)?,
    )
  }

  /// Records `lhs = rhs` as `(lhs - rhs) * 1 = 0`.
  ///
  /// # Errors
  ///
  /// Returns an error if any referenced cell is missing from the canonical
  /// assignment map.
  pub fn add_linear_gate(
    &mut self,
    lhs: &Halo2CellLinearCombination,
    rhs: &Halo2CellLinearCombination,
  ) -> Result<(), R1csBuildError> {
    self.builder.add_linear_constraint(
      &self.lower_linear_combination(lhs)?,
      &self.lower_linear_combination(rhs)?,
    )
  }

  /// Records `lhs = constant` as `(lhs - constant) * 1 = 0`.
  ///
  /// # Errors
  ///
  /// Returns an error if any referenced cell is missing from the canonical
  /// assignment map.
  pub fn add_linear_constant_gate(
    &mut self,
    lhs: &Halo2CellLinearCombination,
    constant: NativeField,
  ) -> Result<(), R1csBuildError> {
    self.builder.add_linear_constant_constraint(&self.lower_linear_combination(lhs)?, constant)
  }

  /// Finalizes the canonical R1CS circuit.
  #[must_use]
  pub fn build(self) -> R1csCircuit {
    self.builder.build()
  }

  fn lower_linear_combination(
    &self,
    linear_combination: &Halo2CellLinearCombination,
  ) -> Result<LinearCombination, R1csBuildError> {
    let terms = linear_combination
      .terms
      .iter()
      .map(|term| Ok(LinearTerm::new(self.variable_for(term.cell)?, term.coeff)))
      .collect::<Result<Vec<_>, R1csBuildError>>()?;

    Ok(LinearCombination::new(terms, linear_combination.constant))
  }
}
