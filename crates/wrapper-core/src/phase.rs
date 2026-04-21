//! Phase and status reporting types.

use serde::{Deserialize, Serialize};

use crate::capabilities::CapabilityMatrix;

/// Current project phase.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectPhase {
  /// Repository bootstrap and architecture setup.
  Initialization,
  /// Future stage for early circuit-facing work.
  #[default]
  Stage1,
  /// Future stage for pairing-related research and implementation.
  PairingResearch,
  /// Future stage for wrapper verifier implementation.
  WrapperVerifier,
}

/// Diagnostic report for the repository state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectStatusReport {
  /// Current phase.
  pub phase: ProjectPhase,
  /// Honest capability matrix.
  pub capabilities: CapabilityMatrix,
  /// Short limitations that still apply.
  pub limitations: Vec<String>,
}

impl ProjectStatusReport {
  /// Returns the current repository status report.
  #[must_use]
  pub fn scaffold() -> Self {
    Self {
      phase: ProjectPhase::Stage1,
      capabilities: CapabilityMatrix::scaffolded(),
      limitations: vec![
        "bn254 fp, fp2, g1, and narrow g2 affine/projective support are still early-stage foundations only"
          .to_owned(),
        "no subgroup checks beyond the selected g1 construction path".to_owned(),
        "g2 support is still limited to affine checks plus incomplete Jacobian from_affine/neg/double/add primitives"
          .to_owned(),
        "no fp6 or fp12 support".to_owned(),
        "no pairings".to_owned(),
        "no Groth16 verifier logic".to_owned(),
        "no wrapper verifier circuit beyond the narrow primitive layer".to_owned(),
      ],
    }
  }
}
