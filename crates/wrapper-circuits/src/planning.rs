//! Planning-oriented views for future circuit layout work.

use wrapper_core::{LayoutDescriptor, ProjectConfig};

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
}
