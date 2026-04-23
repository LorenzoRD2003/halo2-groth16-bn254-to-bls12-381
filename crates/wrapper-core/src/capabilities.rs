//! Capability declarations for honest status reporting.

use serde::{Deserialize, Serialize};

/// Implementation maturity for a capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImplementationStatus {
  /// The capability is intentionally absent in the current phase.
  NotImplemented,
  /// The capability exists only as an interface or module shell.
  Scaffolded,
  /// The capability is partially implemented.
  Experimental,
  /// The capability is considered implemented for the current stage.
  Implemented,
}

/// A high-level capability that contributors may care about.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectCapability {
  /// Configuration loading and validation.
  ConfigModel,
  /// Halo2-facing circuit shell structure.
  CircuitSkeleton,
  /// Artifact parsing and backend integration points.
  BackendSkeleton,
  /// Foreign field arithmetic support.
  ForeignFieldArithmetic,
  /// Elliptic curve gadget support.
  EccGadgets,
  /// Pairing gadget support.
  PairingGadgets,
  /// Groth16 verifier integration.
  Groth16Verifier,
}

/// Capability report exposed by diagnostics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapabilityMatrix {
  /// Capability entries and their implementation status.
  pub entries: Vec<(ProjectCapability, ImplementationStatus)>,
}

impl CapabilityMatrix {
  /// Returns the scaffolded capability matrix for the current repository state.
  #[must_use]
  pub fn scaffolded() -> Self {
    Self {
      entries: vec![
        (ProjectCapability::ConfigModel, ImplementationStatus::Implemented),
        (ProjectCapability::CircuitSkeleton, ImplementationStatus::Experimental),
        (ProjectCapability::BackendSkeleton, ImplementationStatus::Scaffolded),
        (ProjectCapability::ForeignFieldArithmetic, ImplementationStatus::Experimental),
        (ProjectCapability::EccGadgets, ImplementationStatus::Experimental),
        (ProjectCapability::PairingGadgets, ImplementationStatus::Experimental),
        (ProjectCapability::Groth16Verifier, ImplementationStatus::Experimental),
      ],
    }
  }
}
