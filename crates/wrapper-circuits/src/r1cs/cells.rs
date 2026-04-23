use ff::Field;

use crate::NativeField;

/// Deterministic identity for one Halo2 cell.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Halo2CellRef {
  /// Cell in an instance column.
  Instance {
    /// Zero-based instance-column index.
    column: usize,
    /// Zero-based row index.
    row: usize,
  },
  /// Cell in an advice column.
  Advice {
    /// Zero-based advice-column index.
    column: usize,
    /// Zero-based row index.
    row: usize,
  },
}

/// One Halo2 equality/copy edge.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct EqualityEdge {
  /// Left endpoint of the copy edge.
  pub left: Halo2CellRef,
  /// Right endpoint of the copy edge.
  pub right: Halo2CellRef,
}

impl EqualityEdge {
  /// Creates a normalized equality edge.
  #[must_use]
  pub fn new(left: Halo2CellRef, right: Halo2CellRef) -> Self {
    if left <= right { Self { left, right } } else { Self { left: right, right: left } }
  }

  /// Returns this edge in normalized endpoint order.
  #[must_use]
  pub fn normalized(self) -> Self {
    Self::new(self.left, self.right)
  }
}

/// Canonical identifier for one Halo2 equality class.
///
/// The class id is the canonical representative of the class, which is always
/// the minimum `Halo2CellRef` in the class.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalClassId(pub Halo2CellRef);

impl CanonicalClassId {
  /// Creates a canonical class id from its representative.
  #[must_use]
  pub fn new(representative: Halo2CellRef) -> Self {
    Self(representative)
  }

  /// Returns the representative cell for this class.
  #[must_use]
  pub fn representative(self) -> Halo2CellRef {
    self.0
  }

  /// Returns whether this class corresponds to a public-input variable.
  #[must_use]
  pub fn is_public(self) -> bool {
    matches!(self.representative(), Halo2CellRef::Instance { .. })
  }
}

/// One sparse Halo2-cell linear-combination term.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Halo2CellTerm {
  /// Cell referenced by this term.
  pub cell: Halo2CellRef,
  /// Coefficient applied to the cell.
  pub coeff: NativeField,
}

impl Halo2CellTerm {
  /// Creates a new sparse Halo2-cell term.
  #[must_use]
  pub fn new(cell: Halo2CellRef, coeff: NativeField) -> Self {
    Self { cell, coeff }
  }
}

/// Sparse Halo2-cell linear combination before variable assignment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Halo2CellLinearCombination {
  /// Sparse non-constant terms.
  pub terms: Vec<Halo2CellTerm>,
  /// Explicit coefficient on the implicit constant-one slot.
  pub constant: NativeField,
}

impl Halo2CellLinearCombination {
  /// Creates a normalized sparse Halo2-cell linear combination.
  #[must_use]
  pub fn new(terms: Vec<Halo2CellTerm>, constant: NativeField) -> Self {
    let mut normalized_terms = terms;
    normalized_terms.sort_by_key(|term| term.cell);

    let mut combined: Vec<Halo2CellTerm> = Vec::with_capacity(normalized_terms.len());
    for term in normalized_terms {
      if term.coeff == NativeField::ZERO {
        continue;
      }

      if let Some(previous) = combined.last_mut()
        && previous.cell == term.cell
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

  /// Returns a constant-only linear combination.
  #[must_use]
  pub fn constant(constant: NativeField) -> Self {
    Self::new(Vec::new(), constant)
  }

  /// Returns a linear combination containing one cell with coefficient 1.
  #[must_use]
  pub fn from_cell(cell: Halo2CellRef) -> Self {
    Self::new(vec![Halo2CellTerm::new(cell, NativeField::ONE)], NativeField::ZERO)
  }
}
