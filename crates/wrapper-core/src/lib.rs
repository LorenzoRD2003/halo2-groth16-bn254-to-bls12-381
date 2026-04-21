//! Core domain types and architectural contracts for the wrapper workspace.
//!
//! This crate is intentionally light on proving-system dependencies. It models
//! configuration, metadata, capability declarations, and stable interfaces that
//! other crates can build on during later stages.

pub mod capabilities;
pub mod config;
pub mod error;
pub mod layout;
pub mod metadata;
pub mod phase;

pub use capabilities::{CapabilityMatrix, ImplementationStatus, ProjectCapability};
pub use config::{ProjectConfig, ProjectPaths, WrapperFlavor, WrapperParameters};
pub use error::{ConfigError, WrapperError};
pub use layout::{LayoutComponentKind, LayoutDescriptor, LayoutNode};
pub use metadata::{
  NormalizedProofArtifact, NormalizedVerificationKey, ProofSystemDescriptor, ProofSystemKind,
};
pub use phase::{ProjectPhase, ProjectStatusReport};

#[cfg(test)]
mod tests {
  use super::{ProjectConfig, ProjectPhase};

  #[test]
  fn default_config_uses_initialization_phase() {
    let config = ProjectConfig::default();

    assert_eq!(config.phase, ProjectPhase::Initialization);
  }
}
