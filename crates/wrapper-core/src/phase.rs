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
      phase: ProjectPhase::WrapperVerifier,
      capabilities: CapabilityMatrix::scaffolded(),
      limitations: vec![
        "bn254 fp, fp2, fp6, fp12, g1, and narrow g2 affine/projective support are still early-stage foundations only"
          .to_owned(),
        "no subgroup checks beyond the selected g1 construction path".to_owned(),
        "g2 support is still limited to affine checks, narrow Jacobian from_affine/neg/double/add primitives, and Miller-path double_with_line/mixed_add_with_line steps"
          .to_owned(),
        "pairing support remains intentionally narrow: real Miller loop, final exponentiation, verifier-shaped pairing checks, and the first Groth16 verifier reduction".to_owned(),
        "Groth16 verification is currently limited to the first narrow BN254 slice: snarkjs fixture parsing, IC accumulation, and one product-check verification path".to_owned(),
        "there is still no broad wrapper-verifier orchestration, subgroup coverage, or production serialization ecosystem support".to_owned(),
      ],
    }
  }
}
