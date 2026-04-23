//! Planning-oriented views for current and future circuit layout work.

use std::fmt;

use wrapper_core::{LayoutDescriptor, ProjectConfig};

use crate::{
  CostEstimate, LayoutMetrics, final_exponentiation_layout_metrics, fp_add_layout_metrics,
  fp_mul_layout_metrics, fp2_add_layout_metrics, fp2_mul_layout_metrics, fp2_square_layout_metrics,
  fp6_add_layout_metrics, fp6_mul_layout_metrics, fp6_square_layout_metrics,
  fp12_add_layout_metrics, fp12_cyclotomic_square_layout_metrics, fp12_mul_layout_metrics,
  fp12_square_layout_metrics, g1_add_layout_metrics, g2_double_with_line_layout_metrics,
  g2_mixed_add_with_line_layout_metrics, g2_neg_layout_metrics, g2_on_curve_layout_metrics,
  g2_proj_add_layout_metrics, g2_proj_double_layout_metrics, g2_proj_from_affine_layout_metrics,
  miller_accumulator_mul_by_line_layout_metrics,
  miller_accumulator_mul_by_line_sparse_layout_metrics, miller_accumulator_square_layout_metrics,
  miller_loop_layout_metrics, pairing_check_layout_metrics,
};

/// Number of currently measured primitive circuits.
pub const PRIMITIVE_COUNT: usize = 26;

/// High-level layer for a measured primitive cost entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrimitiveCostLayer {
  /// Base and extension field arithmetic.
  Field,
  /// Curve arithmetic and validation.
  Curve,
  /// Miller-preparation steps and line extraction.
  MillerPrep,
  /// Miller accumulation and fixed-schedule loop driving.
  MillerLoop,
}

impl fmt::Display for PrimitiveCostLayer {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Field => formatter.write_str("Field"),
      Self::Curve => formatter.write_str("Curve"),
      Self::MillerPrep => formatter.write_str("Miller-Prep"),
      Self::MillerLoop => formatter.write_str("Miller-Loop"),
    }
  }
}

/// Canonical metadata for one measured primitive.
#[derive(Clone, Copy, Debug)]
pub struct PrimitiveDefinition {
  /// Stable identifier for the primitive.
  pub key: &'static str,
  /// Human-facing display label.
  pub label: &'static str,
  /// High-level layer grouping.
  pub layer: PrimitiveCostLayer,
  /// Criterion bench module path segment.
  pub bench_module: &'static str,
  /// Criterion bench entry point name.
  pub bench_name: &'static str,
  /// Whether CLI output should include lookup counts for this primitive.
  pub show_lookups: bool,
  measure_layout: fn() -> LayoutMetrics,
}

impl PrimitiveDefinition {
  /// Measures the primitive's current layout metrics.
  #[must_use]
  pub fn layout_metrics(self) -> LayoutMetrics {
    (self.measure_layout)()
  }
}

const PRIMITIVE_DEFINITIONS: [PrimitiveDefinition; PRIMITIVE_COUNT] = [
  PrimitiveDefinition {
    key: "fp_add",
    label: "fp add",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp_add",
    show_lookups: false,
    measure_layout: fp_add_layout_metrics,
  },
  PrimitiveDefinition {
    key: "fp_mul",
    label: "fp mul",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp_mul",
    show_lookups: false,
    measure_layout: fp_mul_layout_metrics,
  },
  PrimitiveDefinition {
    key: "fp2_add",
    label: "fp2 add",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp2_add",
    show_lookups: false,
    measure_layout: fp2_add_layout_metrics,
  },
  PrimitiveDefinition {
    key: "fp2_mul",
    label: "fp2 mul",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp2_mul",
    show_lookups: false,
    measure_layout: fp2_mul_layout_metrics,
  },
  PrimitiveDefinition {
    key: "fp2_square",
    label: "fp2 square",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp2_square",
    show_lookups: false,
    measure_layout: fp2_square_layout_metrics,
  },
  PrimitiveDefinition {
    key: "fp6_add",
    label: "fp6 add",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp6_add",
    show_lookups: false,
    measure_layout: fp6_add_layout_metrics,
  },
  PrimitiveDefinition {
    key: "fp6_mul",
    label: "fp6 mul",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp6_mul",
    show_lookups: false,
    measure_layout: fp6_mul_layout_metrics,
  },
  PrimitiveDefinition {
    key: "fp6_square",
    label: "fp6 square",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp6_square",
    show_lookups: false,
    measure_layout: fp6_square_layout_metrics,
  },
  PrimitiveDefinition {
    key: "fp12_add",
    label: "fp12 add",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp12_add",
    show_lookups: false,
    measure_layout: fp12_add_layout_metrics,
  },
  PrimitiveDefinition {
    key: "fp12_mul",
    label: "fp12 mul",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp12_mul",
    show_lookups: false,
    measure_layout: fp12_mul_layout_metrics,
  },
  PrimitiveDefinition {
    key: "fp12_square",
    label: "fp12 square",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp12_square",
    show_lookups: false,
    measure_layout: fp12_square_layout_metrics,
  },
  PrimitiveDefinition {
    key: "fp12_cyclotomic_square",
    label: "fp12 cyclotomic square",
    layer: PrimitiveCostLayer::Field,
    bench_module: "field",
    bench_name: "bench_fp12_cyclotomic_square",
    show_lookups: false,
    measure_layout: fp12_cyclotomic_square_layout_metrics,
  },
  PrimitiveDefinition {
    key: "g1_add",
    label: "g1 add",
    layer: PrimitiveCostLayer::Curve,
    bench_module: "ecc",
    bench_name: "bench_g1_add",
    show_lookups: false,
    measure_layout: g1_add_layout_metrics,
  },
  PrimitiveDefinition {
    key: "g2_on_curve",
    label: "g2 on_curve",
    layer: PrimitiveCostLayer::Curve,
    bench_module: "ecc",
    bench_name: "bench_g2_on_curve",
    show_lookups: false,
    measure_layout: g2_on_curve_layout_metrics,
  },
  PrimitiveDefinition {
    key: "g2_neg",
    label: "g2 neg",
    layer: PrimitiveCostLayer::Curve,
    bench_module: "ecc",
    bench_name: "bench_g2_neg",
    show_lookups: false,
    measure_layout: g2_neg_layout_metrics,
  },
  PrimitiveDefinition {
    key: "g2_proj_from_affine",
    label: "g2 proj from_affine",
    layer: PrimitiveCostLayer::Curve,
    bench_module: "ecc",
    bench_name: "bench_g2_proj_from_affine",
    show_lookups: false,
    measure_layout: g2_proj_from_affine_layout_metrics,
  },
  PrimitiveDefinition {
    key: "g2_proj_double",
    label: "g2 proj double",
    layer: PrimitiveCostLayer::Curve,
    bench_module: "ecc",
    bench_name: "bench_g2_proj_double",
    show_lookups: false,
    measure_layout: g2_proj_double_layout_metrics,
  },
  PrimitiveDefinition {
    key: "g2_proj_add",
    label: "g2 proj add",
    layer: PrimitiveCostLayer::Curve,
    bench_module: "ecc",
    bench_name: "bench_g2_proj_add",
    show_lookups: false,
    measure_layout: g2_proj_add_layout_metrics,
  },
  PrimitiveDefinition {
    key: "g2_double_with_line",
    label: "g2 double_with_line",
    layer: PrimitiveCostLayer::MillerPrep,
    bench_module: "ecc",
    bench_name: "bench_g2_double_with_line",
    show_lookups: false,
    measure_layout: g2_double_with_line_layout_metrics,
  },
  PrimitiveDefinition {
    key: "g2_mixed_add_with_line",
    label: "g2 mixed_add_with_line",
    layer: PrimitiveCostLayer::MillerPrep,
    bench_module: "ecc",
    bench_name: "bench_g2_mixed_add_with_line",
    show_lookups: false,
    measure_layout: g2_mixed_add_with_line_layout_metrics,
  },
  PrimitiveDefinition {
    key: "miller_accumulator_square",
    label: "miller accumulator square",
    layer: PrimitiveCostLayer::MillerLoop,
    bench_module: "ecc",
    bench_name: "bench_miller_accumulator_square",
    show_lookups: false,
    measure_layout: miller_accumulator_square_layout_metrics,
  },
  PrimitiveDefinition {
    key: "miller_accumulator_mul_by_line",
    label: "miller accumulator mul_by_line",
    layer: PrimitiveCostLayer::MillerLoop,
    bench_module: "ecc",
    bench_name: "bench_miller_accumulator_mul_by_line",
    show_lookups: false,
    measure_layout: miller_accumulator_mul_by_line_layout_metrics,
  },
  PrimitiveDefinition {
    key: "miller_accumulator_mul_by_line_sparse",
    label: "miller accumulator mul_by_line sparse",
    layer: PrimitiveCostLayer::MillerLoop,
    bench_module: "ecc",
    bench_name: "bench_miller_accumulator_mul_by_line_sparse",
    show_lookups: false,
    measure_layout: miller_accumulator_mul_by_line_sparse_layout_metrics,
  },
  PrimitiveDefinition {
    key: "miller_loop",
    label: "miller loop narrow",
    layer: PrimitiveCostLayer::MillerLoop,
    bench_module: "ecc",
    bench_name: "bench_miller_loop_narrow",
    show_lookups: false,
    measure_layout: miller_loop_layout_metrics,
  },
  PrimitiveDefinition {
    key: "final_exponentiation",
    label: "final exponentiation",
    layer: PrimitiveCostLayer::MillerLoop,
    bench_module: "ecc",
    bench_name: "bench_final_exponentiation",
    show_lookups: false,
    measure_layout: final_exponentiation_layout_metrics,
  },
  PrimitiveDefinition {
    key: "pairing_check",
    label: "pairing check",
    layer: PrimitiveCostLayer::MillerLoop,
    bench_module: "ecc",
    bench_name: "bench_pairing_check",
    show_lookups: false,
    measure_layout: pairing_check_layout_metrics,
  },
];

/// Returns the canonical primitive registry for planning, CLI, and benchmarks.
#[must_use]
pub fn primitive_definitions() -> &'static [PrimitiveDefinition; PRIMITIVE_COUNT] {
  &PRIMITIVE_DEFINITIONS
}

/// A single measured primitive cost record for CLI/reporting consumers.
#[derive(Clone, Copy, Debug)]
pub struct PrimitiveCostEntry {
  /// Static primitive metadata.
  pub definition: PrimitiveDefinition,
  /// Raw layout metrics for the measured circuit.
  pub layout: LayoutMetrics,
}

impl PrimitiveCostEntry {
  fn from_definition(definition: PrimitiveDefinition) -> Self {
    Self { definition, layout: definition.layout_metrics() }
  }

  /// Cost summary derived from the measured layout.
  #[must_use]
  pub fn cost(&self) -> CostEstimate {
    self.layout.cost_estimate()
  }
}

/// Measured primitive costs for the current BN254 slice.
#[derive(Clone, Copy, Debug)]
pub struct PrimitiveCostTable {
  entries: [PrimitiveCostEntry; PRIMITIVE_COUNT],
}

impl PrimitiveCostTable {
  /// Builds the table from the canonical primitive registry.
  #[must_use]
  pub fn current() -> Self {
    Self { entries: PRIMITIVE_DEFINITIONS.map(PrimitiveCostEntry::from_definition) }
  }

  /// Returns the measured primitive costs as a single canonical registry.
  #[must_use]
  pub fn entries(&self) -> &[PrimitiveCostEntry; PRIMITIVE_COUNT] {
    &self.entries
  }
}

/// Read-only planning view for CLI inspection.
#[derive(Clone, Copy, Debug, Default)]
pub struct CircuitPlanningView;

impl CircuitPlanningView {
  /// Creates a planning view from project config.
  #[must_use]
  pub fn from_config(_config: ProjectConfig) -> Self {
    Self
  }

  /// Returns the scaffold layout tree.
  #[must_use]
  pub fn describe(&self) -> LayoutDescriptor {
    LayoutDescriptor::scaffold()
  }

  /// Returns measured layout metrics for the current primitive layer.
  #[must_use]
  pub fn primitive_cost_table(&self) -> PrimitiveCostTable {
    PrimitiveCostTable::current()
  }
}
