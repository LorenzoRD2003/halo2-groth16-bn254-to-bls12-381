//! Core domain types and architectural contracts for the wrapper workspace.
//!
//! This crate is intentionally light on proving-system dependencies. It models
//! configuration, metadata, capability declarations, and stable interfaces that
//! other crates can build on during later stages.

pub mod capabilities;
pub mod config;
pub mod error;
pub mod execution;
pub mod job;
pub mod layout;
pub mod metadata;
pub mod output;
pub mod package;
pub mod phase;
pub mod statement;

pub use capabilities::{CapabilityMatrix, ImplementationStatus, ProjectCapability};
pub use config::{ProjectConfig, ProjectPaths, WrapperFlavor, WrapperParameters};
pub use error::{ConfigError, WrapperError};
pub use execution::{WrapperExecutionResult, WrapperExecutionStatus};
pub use job::WrapperJob;
pub use layout::{LayoutComponentKind, LayoutDescriptor, LayoutNode};
pub use metadata::{
  NormalizedProofArtifact, NormalizedVerificationKey, ProofSystemDescriptor, ProofSystemKind,
};
pub use output::{
  ExpectedProofArtifactShape, ExpectedPublicInputsArtifactShape,
  ExpectedVerificationKeyArtifactShape, ExpectedWrapperArtifacts,
};
pub use package::{WrapperExecutionPackage, WrapperStatement, WrapperWitnessInput};
pub use phase::{ProjectOverview, ProjectPhase, ProjectStatusReport};
pub use statement::{NamedPublicInput, NamedPublicInputs};

#[cfg(test)]
mod tests {
  use super::{ProjectConfig, ProjectPhase, ProjectStatusReport};

  #[test]
  fn default_config_uses_stage1_phase() {
    let config = ProjectConfig::default();

    assert_eq!(config.phase, ProjectPhase::Stage1);
  }

  #[test]
  fn project_overview_mentions_week5_verifier_slice() {
    let overview = ProjectStatusReport::overview();

    assert!(overview.phase_label.contains("week 5"));
    assert!(overview.current_implementation.contains("Groth16 BN254 verifier"));
    assert!(overview.not_implemented.contains("proof generation"));
  }
}
