//! Planning-oriented views for current and future circuit layout work.

use wrapper_core::{LayoutDescriptor, ProjectConfig};

use crate::{
  CostEstimate, LayoutMetrics, fp_add_layout_metrics, fp_mul_layout_metrics,
  fp2_add_layout_metrics, fp2_mul_layout_metrics, fp2_square_layout_metrics, g1_add_layout_metrics,
  g2_neg_layout_metrics, g2_on_curve_layout_metrics, g2_proj_add_layout_metrics,
  g2_proj_double_layout_metrics, g2_proj_from_affine_layout_metrics,
};

/// Layout and cost data for the currently implemented primitive layer.
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
  /// Fp2 addition layout metrics.
  pub fp2_add_layout: LayoutMetrics,
  /// Fp2 addition cost summary.
  pub fp2_add: CostEstimate,
  /// Fp2 multiplication layout metrics.
  pub fp2_mul_layout: LayoutMetrics,
  /// Fp2 multiplication cost summary.
  pub fp2_mul: CostEstimate,
  /// Fp2 square layout metrics.
  pub fp2_square_layout: LayoutMetrics,
  /// Fp2 square cost summary.
  pub fp2_square: CostEstimate,
  /// G1 addition layout metrics.
  pub g1_add_layout: LayoutMetrics,
  /// G1 addition cost summary.
  pub g1_add: CostEstimate,
  /// G2 on-curve layout metrics.
  pub g2_on_curve_layout: LayoutMetrics,
  /// G2 on-curve cost summary.
  pub g2_on_curve: CostEstimate,
  /// G2 negation layout metrics.
  pub g2_neg_layout: LayoutMetrics,
  /// G2 negation cost summary.
  pub g2_neg: CostEstimate,
  /// G2 affine-to-projective embedding layout metrics.
  pub g2_proj_from_affine_layout: LayoutMetrics,
  /// G2 affine-to-projective embedding cost summary.
  pub g2_proj_from_affine: CostEstimate,
  /// G2 projective doubling layout metrics.
  pub g2_proj_double_layout: LayoutMetrics,
  /// G2 projective doubling cost summary.
  pub g2_proj_double: CostEstimate,
  /// G2 projective addition layout metrics.
  pub g2_proj_add_layout: LayoutMetrics,
  /// G2 projective addition cost summary.
  pub g2_proj_add: CostEstimate,
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

  /// Returns measured layout metrics for the current primitive layer.
  #[must_use]
  pub fn primitive_cost_table(&self) -> PrimitiveCostTable {
    let _ = &self.config;
    let base_field_add_layout = fp_add_layout_metrics();
    let base_field_mul_layout = fp_mul_layout_metrics();
    let quadratic_field_add_layout = fp2_add_layout_metrics();
    let quadratic_field_mul_layout = fp2_mul_layout_metrics();
    let quadratic_field_square_layout = fp2_square_layout_metrics();
    let g1_point_add_layout = g1_add_layout_metrics();
    let g2_affine_on_curve_layout = g2_on_curve_layout_metrics();
    let g2_affine_neg_layout = g2_neg_layout_metrics();
    let g2_projective_from_affine_layout = g2_proj_from_affine_layout_metrics();
    let g2_projective_double_layout = g2_proj_double_layout_metrics();
    let g2_projective_add_layout = g2_proj_add_layout_metrics();

    PrimitiveCostTable {
      fp_add: base_field_add_layout.cost_estimate(),
      fp_add_layout: base_field_add_layout,
      fp_mul: base_field_mul_layout.cost_estimate(),
      fp_mul_layout: base_field_mul_layout,
      fp2_add: quadratic_field_add_layout.cost_estimate(),
      fp2_add_layout: quadratic_field_add_layout,
      fp2_mul: quadratic_field_mul_layout.cost_estimate(),
      fp2_mul_layout: quadratic_field_mul_layout,
      fp2_square: quadratic_field_square_layout.cost_estimate(),
      fp2_square_layout: quadratic_field_square_layout,
      g1_add: g1_point_add_layout.cost_estimate(),
      g1_add_layout: g1_point_add_layout,
      g2_on_curve: g2_affine_on_curve_layout.cost_estimate(),
      g2_on_curve_layout: g2_affine_on_curve_layout,
      g2_neg: g2_affine_neg_layout.cost_estimate(),
      g2_neg_layout: g2_affine_neg_layout,
      g2_proj_from_affine: g2_projective_from_affine_layout.cost_estimate(),
      g2_proj_from_affine_layout: g2_projective_from_affine_layout,
      g2_proj_double: g2_projective_double_layout.cost_estimate(),
      g2_proj_double_layout: g2_projective_double_layout,
      g2_proj_add: g2_projective_add_layout.cost_estimate(),
      g2_proj_add_layout: g2_projective_add_layout,
    }
  }
}
