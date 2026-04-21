//! Outer wrapper circuit placeholders.

use wrapper_core::{LayoutDescriptor, ProjectConfig, WrapperError};

/// Build status for the outer circuit shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CircuitBuildStatus {
  /// Week 1 primitive layers are available, but no wrapper verifier exists yet.
  Week1Foundation,
}

/// Placeholder outer wrapper circuit definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterWrapperCircuit {
  /// Config used to describe the intended circuit.
  pub config: ProjectConfig,
}

impl OuterWrapperCircuit {
  /// Creates a new outer wrapper circuit placeholder.
  #[must_use]
  pub fn new(config: ProjectConfig) -> Self {
    Self { config }
  }

  /// Returns the current build status.
  #[must_use]
  pub fn build_status(&self) -> CircuitBuildStatus {
    CircuitBuildStatus::Week1Foundation
  }

  /// Returns the scaffold layout for reporting purposes.
  #[must_use]
  pub fn layout_descriptor(&self) -> LayoutDescriptor {
    LayoutDescriptor::scaffold()
  }

  /// Placeholder hook for future synthesis.
  ///
  /// # Errors
  ///
  /// Always returns an error during the current Week 1 phase because outer
  /// wrapper synthesis is intentionally not implemented yet.
  pub fn assert_ready_for_synthesis(&self) -> Result<(), WrapperError> {
    let _ = self;
    Err(WrapperError::NotImplemented(
      "outer wrapper synthesis is not available during the current week-1 foundation phase",
    ))
  }
}
