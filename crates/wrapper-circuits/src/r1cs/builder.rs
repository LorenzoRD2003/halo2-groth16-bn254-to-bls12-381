use super::{LinearCombination, R1csBuildError, R1csCircuit, R1csConstraint, VariableId};
use crate::NativeField;

/// Deterministic builder for the canonical R1CS subset.
#[derive(Clone, Debug, Default)]
pub struct CanonicalR1csBuilder {
  public_inputs: Vec<VariableId>,
  witnesses: Vec<VariableId>,
  constraints: Vec<R1csConstraint>,
  next_variable_index: u32,
}

impl CanonicalR1csBuilder {
  /// Creates an empty canonical R1CS builder.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Creates a builder from an existing canonical variable partition.
  ///
  /// # Errors
  ///
  /// Returns an error if the provided variable partition is not contiguous and
  /// canonical.
  pub fn from_variable_partition(
    public_inputs: Vec<VariableId>,
    witnesses: Vec<VariableId>,
  ) -> Result<Self, R1csBuildError> {
    let total_variable_count = public_inputs.len() + witnesses.len();
    let mut seen = vec![false; total_variable_count];
    for var in public_inputs.iter().chain(&witnesses) {
      let index = var.index() as usize;
      if index >= total_variable_count || std::mem::replace(&mut seen[index], true) {
        return Err(R1csBuildError::InconsistentEquality);
      }
    }

    if seen.into_iter().any(|present| !present) {
      return Err(R1csBuildError::InconsistentEquality);
    }

    Ok(Self {
      next_variable_index: total_variable_count as u32,
      public_inputs,
      witnesses,
      constraints: Vec::new(),
    })
  }

  /// Allocates one canonical public-input variable.
  pub fn add_public_input(&mut self) -> VariableId {
    let var = self.allocate_variable();
    self.public_inputs.push(var);
    var
  }

  /// Allocates one canonical witness variable.
  pub fn add_witness(&mut self) -> VariableId {
    let var = self.allocate_variable();
    self.witnesses.push(var);
    var
  }

  /// Inserts one already-shaped R1CS constraint after validation.
  ///
  /// # Errors
  ///
  /// Returns an error if any linear combination references an undeclared
  /// variable.
  pub fn add_constraint(
    &mut self,
    a: LinearCombination,
    b: LinearCombination,
    c: LinearCombination,
  ) -> Result<(), R1csBuildError> {
    self.assert_declared_linear_combination(&a)?;
    self.assert_declared_linear_combination(&b)?;
    self.assert_declared_linear_combination(&c)?;
    self.constraints.push(R1csConstraint::new(a, b, c));
    Ok(())
  }

  /// Lowers `lhs * rhs = output` into canonical R1CS.
  ///
  /// # Errors
  ///
  /// Returns an error if any referenced variable has not been allocated.
  pub fn add_multiplication_constraint(
    &mut self,
    lhs: VariableId,
    rhs: VariableId,
    output: VariableId,
  ) -> Result<(), R1csBuildError> {
    self.add_constraint(
      LinearCombination::from_var(lhs),
      LinearCombination::from_var(rhs),
      LinearCombination::from_var(output),
    )
  }

  /// Lowers `var * constant = output` into canonical R1CS.
  ///
  /// # Errors
  ///
  /// Returns an error if any referenced variable has not been allocated.
  pub fn add_scaled_multiplication_constraint(
    &mut self,
    var: VariableId,
    constant: NativeField,
    output: VariableId,
  ) -> Result<(), R1csBuildError> {
    self.add_constraint(
      LinearCombination::from_var(var),
      LinearCombination::constant(constant),
      LinearCombination::from_var(output),
    )
  }

  /// Lowers `lhs = rhs` as `(lhs - rhs) * 1 = 0`.
  ///
  /// # Errors
  ///
  /// Returns an error if any referenced variable has not been allocated.
  pub fn add_linear_constraint(
    &mut self,
    lhs: &LinearCombination,
    rhs: &LinearCombination,
  ) -> Result<(), R1csBuildError> {
    self.add_constraint(
      LinearCombination::difference(lhs, rhs),
      LinearCombination::one(),
      LinearCombination::zero(),
    )
  }

  /// Lowers `lhs = constant` as `(lhs - constant) * 1 = 0`.
  ///
  /// # Errors
  ///
  /// Returns an error if any referenced variable has not been allocated.
  pub fn add_linear_constant_constraint(
    &mut self,
    lhs: &LinearCombination,
    constant: NativeField,
  ) -> Result<(), R1csBuildError> {
    self.add_linear_constraint(lhs, &LinearCombination::constant(constant))
  }

  /// Finalizes the canonical R1CS circuit.
  #[must_use]
  pub fn build(self) -> R1csCircuit {
    R1csCircuit {
      public_inputs: self.public_inputs,
      witnesses: self.witnesses,
      constraints: self.constraints,
    }
  }

  fn allocate_variable(&mut self) -> VariableId {
    let var = VariableId(self.next_variable_index);
    self.next_variable_index += 1;
    var
  }

  fn assert_declared_linear_combination(
    &self,
    linear_combination: &LinearCombination,
  ) -> Result<(), R1csBuildError> {
    for term in &linear_combination.terms {
      if term.var.index() >= self.next_variable_index {
        return Err(R1csBuildError::UndeclaredVariable { var: term.var });
      }
    }
    Ok(())
  }
}
