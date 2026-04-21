//! Lightweight layout and cost reporting for Week 1 primitives.

use std::ops::{Add, AddAssign};

use midnight_proofs::dev::cost_model::CircuitModel;

/// Rough row and constraint counts for a primitive operation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CostEstimate {
  /// Measured or estimated row usage.
  pub rows: usize,
  /// Rough constraint proxy derived from column queries.
  pub constraints: usize,
}

impl CostEstimate {
  /// Creates a new cost estimate.
  #[must_use]
  pub const fn new(rows: usize, constraints: usize) -> Self {
    Self { rows, constraints }
  }
}

impl Add for CostEstimate {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    Self { rows: self.rows + rhs.rows, constraints: self.constraints + rhs.constraints }
  }
}

impl AddAssign for CostEstimate {
  fn add_assign(&mut self, rhs: Self) {
    self.rows += rhs.rows;
    self.constraints += rhs.constraints;
  }
}

/// Real layout metrics collected from a Halo2 circuit model.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutMetrics {
  /// Power-of-two domain size bound.
  pub k: u32,
  /// Number of computation rows.
  pub rows: usize,
  /// Number of lookup table rows.
  pub table_rows: usize,
  /// Maximum gate degree.
  pub max_degree: usize,
  /// Number of advice columns.
  pub advice_columns: usize,
  /// Number of fixed columns.
  pub fixed_columns: usize,
  /// Number of lookup arguments.
  pub lookups: usize,
  /// Number of permutation-enabled columns.
  pub permutations: usize,
  /// Number of distinct column queries.
  pub column_queries: usize,
  /// Number of distinct point sets in the multiopening argument.
  pub point_sets: usize,
}

impl LayoutMetrics {
  /// Converts the full layout metrics into the lightweight CLI-facing estimate.
  #[must_use]
  pub const fn cost_estimate(&self) -> CostEstimate {
    CostEstimate::new(self.rows, self.column_queries)
  }
}

impl From<CircuitModel> for LayoutMetrics {
  fn from(model: CircuitModel) -> Self {
    Self {
      k: model.k,
      rows: model.rows,
      table_rows: model.table_rows,
      max_degree: model.max_deg,
      advice_columns: model.advice_columns,
      fixed_columns: model.fixed_columns,
      lookups: model.lookups,
      permutations: model.permutations,
      column_queries: model.column_queries,
      point_sets: model.point_sets,
    }
  }
}
