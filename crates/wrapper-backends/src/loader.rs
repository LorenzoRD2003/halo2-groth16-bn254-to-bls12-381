//! Placeholder artifact loader interfaces.

use std::fmt;

use wrapper_core::{NormalizedProofArtifact, NormalizedVerificationKey};

/// Summary of what a loader can currently do.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoaderSummary {
  /// Loader name.
  pub name: &'static str,
  /// Whether proof loading is implemented.
  pub proof_loading_available: bool,
  /// Whether verification-key loading is implemented.
  pub vk_loading_available: bool,
}

/// Error raised by placeholder loaders.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactLoaderError {
  feature: &'static str,
}

impl ArtifactLoaderError {
  /// Creates a new "not implemented" loader error.
  #[must_use]
  pub fn not_implemented(feature: &'static str) -> Self {
    Self { feature }
  }
}

impl fmt::Display for ArtifactLoaderError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "backend feature not implemented in the current stage-1 repository state: {}",
      self.feature
    )
  }
}

impl std::error::Error for ArtifactLoaderError {}

/// Backend loader contract for normalized artifacts.
pub trait ArtifactLoader {
  /// Returns a short summary of the loader state.
  fn summary(&self) -> LoaderSummary;

  /// Loads normalized proof metadata.
  ///
  /// # Errors
  ///
  /// Returns an error by default because proof loading is only scaffolded
  /// in the current stage-1 repository state.
  fn load_proof(&self, _input: &[u8]) -> Result<NormalizedProofArtifact, ArtifactLoaderError> {
    Err(ArtifactLoaderError::not_implemented("proof artifact loading"))
  }

  /// Loads normalized verification-key metadata.
  ///
  /// # Errors
  ///
  /// Returns an error by default because verification-key loading is only
  /// scaffolded in the current stage-1 repository state.
  fn load_vk(&self, _input: &[u8]) -> Result<NormalizedVerificationKey, ArtifactLoaderError> {
    Err(ArtifactLoaderError::not_implemented("verification-key loading"))
  }
}
