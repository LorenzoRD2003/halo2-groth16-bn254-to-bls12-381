//! Planning-oriented views for current and future circuit layout work.

use wrapper_core::{LayoutDescriptor, ProjectConfig};

use crate::{
  CostEstimate, LayoutMetrics, fp_add_layout_metrics, fp_mul_layout_metrics, g1_add_layout_metrics,
};

/// Layout and cost data for the currently implemented Week 1 primitives.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PrimitiveCostTable {
  /// Foreign-field addition layout metrics.
  pub fp_add_layout: LayoutMetrics,
  /// Foreign-field addition cost summary.
  pub fp_add: CostEstimate,
  /// Foreign-field multiplication layout metrics.
  pub fp_mul_layout: LayoutMetrics,
  /// Foreign-field multiplication cost summary.
  pub fp_mul: CostEstimate,
  /// G1 addition layout metrics.
  pub g1_add_layout: LayoutMetrics,
  /// G1 addition cost summary.
  pub g1_add: CostEstimate,
}

/// Read-only planning view for CLI inspection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitPlanningView {
  /// Project configuration associated with this view.
  pub config: ProjectConfig,
}

impl CircuitPlanningView {
  /// Creates a planning view from project config.
  #[must_use]
  pub fn from_config(config: ProjectConfig) -> Self {
    Self { config }
  }

  /// Returns the scaffold layout tree.
  #[must_use]
  pub fn describe(&self) -> LayoutDescriptor {
    let _ = &self.config;
    LayoutDescriptor::scaffold()
  }

  /// Returns measured layout metrics for the Week 1 primitive layer.
  #[must_use]
  pub fn primitive_cost_table(&self) -> PrimitiveCostTable {
    let _ = &self.config;
    let fp_add_layout = fp_add_layout_metrics();
    let fp_mul_layout = fp_mul_layout_metrics();
    let g1_add_layout = g1_add_layout_metrics();

    PrimitiveCostTable {
      fp_add: fp_add_layout.cost_estimate(),
      fp_add_layout,
      fp_mul: fp_mul_layout.cost_estimate(),
      fp_mul_layout,
      g1_add: g1_add_layout.cost_estimate(),
      g1_add_layout,
    }
  }
}
