use ff::Field;

use crate::NativeField;

/// Stable identifier for a canonical R1CS variable.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VariableId(pub u32);

impl VariableId {
  /// Returns the zero-based index of this variable in the variable vector.
  #[must_use]
  pub fn index(self) -> u32 {
    self.0
  }
}

/// One sparse linear-combination term.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LinearTerm {
  /// Variable referenced by this term.
  pub var: VariableId,
  /// Coefficient applied to the variable.
  pub coeff: NativeField,
}

impl LinearTerm {
  /// Creates a new sparse linear-combination term.
  #[must_use]
  pub fn new(var: VariableId, coeff: NativeField) -> Self {
    Self { var, coeff }
  }
}

/// Sparse linear combination over the implicit vector `[1, variables...]`.
///
/// The `constant` field stores the coefficient on the implicit constant-one
/// slot explicitly. Terms are always normalized into a deterministic form:
/// sorted by variable id, duplicate variables combined, and zero coefficients
/// removed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinearCombination {
  /// Sparse non-constant terms.
  pub terms: Vec<LinearTerm>,
  /// Explicit coefficient on the implicit constant-one slot.
  pub constant: NativeField,
}

impl LinearCombination {
  /// Creates a normalized sparse linear combination.
  #[must_use]
  pub fn new(terms: Vec<LinearTerm>, constant: NativeField) -> Self {
    let mut normalized_terms = terms;
    normalized_terms.sort_by_key(|term| term.var);

    let mut combined: Vec<LinearTerm> = Vec::with_capacity(normalized_terms.len());
    for term in normalized_terms {
      if term.coeff == NativeField::ZERO {
        continue;
      }

      if let Some(previous) = combined.last_mut()
        && previous.var == term.var
      {
        previous.coeff += term.coeff;
        if previous.coeff == NativeField::ZERO {
          combined.pop();
        }
        continue;
      }

      combined.push(term);
    }

    Self { terms: combined, constant }
  }

  /// Returns the zero linear combination.
  #[must_use]
  pub fn zero() -> Self {
    Self::new(Vec::new(), NativeField::ZERO)
  }

  /// Returns the constant-one linear combination.
  #[must_use]
  pub fn one() -> Self {
    Self::constant(NativeField::ONE)
  }

  /// Returns a linear combination containing one variable with coefficient 1.
  #[must_use]
  pub fn from_var(var: VariableId) -> Self {
    Self::new(vec![LinearTerm::new(var, NativeField::ONE)], NativeField::ZERO)
  }

  /// Returns a constant-only linear combination.
  #[must_use]
  pub fn constant(constant: NativeField) -> Self {
    Self::new(Vec::new(), constant)
  }

  /// Returns `self - other` normalized as one linear combination.
  #[must_use]
  pub fn difference(lhs: &Self, rhs: &Self) -> Self {
    let mut terms = lhs.terms.clone();
    terms.extend(rhs.terms.iter().map(|term| LinearTerm::new(term.var, -term.coeff)));
    Self::new(terms, lhs.constant - rhs.constant)
  }
}

/// Canonical R1CS constraint `(A . X) * (B . X) = (C . X)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct R1csConstraint {
  /// Left multiplicand sparse linear combination.
  pub a: LinearCombination,
  /// Right multiplicand sparse linear combination.
  pub b: LinearCombination,
  /// Output sparse linear combination.
  pub c: LinearCombination,
}

impl R1csConstraint {
  /// Creates a normalized R1CS constraint.
  #[must_use]
  pub fn new(a: LinearCombination, b: LinearCombination, c: LinearCombination) -> Self {
    Self {
      a: LinearCombination::new(a.terms, a.constant),
      b: LinearCombination::new(b.terms, b.constant),
      c: LinearCombination::new(c.terms, c.constant),
    }
  }
}

/// Minimal canonical R1CS circuit representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct R1csCircuit {
  /// Public-input variable ids in canonical order.
  pub public_inputs: Vec<VariableId>,
  /// Witness variable ids in canonical order.
  pub witnesses: Vec<VariableId>,
  /// Constraints in deterministic insertion order.
  pub constraints: Vec<R1csConstraint>,
}

impl R1csCircuit {
  /// Returns the number of non-constant variables in this circuit.
  #[must_use]
  pub fn variable_count(&self) -> usize {
    self.public_inputs.len() + self.witnesses.len()
  }
}
