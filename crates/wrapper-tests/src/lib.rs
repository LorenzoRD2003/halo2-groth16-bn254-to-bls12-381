//! Test harness crate for workspace-level fixtures and integration helpers.
#![allow(clippy::multiple_crate_versions)]

use wrapper_backends as _;
use wrapper_circuits as _;
use wrapper_core as _;

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use midnight_proofs as _;

/// Returns the example config bundled for integration tests.
#[must_use]
pub fn example_config() -> &'static str {
  include_str!("../fixtures/example-config.toml")
}

#[cfg(test)]
mod tests {
  use wrapper_backends::BackendRegistry;
  use wrapper_circuits::CircuitPlanningView;
  use wrapper_core::ProjectConfig;

  use super::example_config;

  #[test]
  fn example_config_parses() {
    let config = ProjectConfig::from_toml_str(example_config()).expect("config should parse");
    let layout = CircuitPlanningView::from_config(config).describe();

    assert_eq!(layout.name, "wrapper-scaffold");
  }

  #[test]
  fn backend_registry_contains_placeholders() {
    let registry = BackendRegistry::scaffold();

    assert_eq!(registry.entries().len(), 2);
  }
}
