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

/// Short human-facing overview for the current repository state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectOverview {
  /// Human-readable current phase summary.
  pub phase_label: String,
  /// Stable short project purpose line.
  pub purpose: String,
  /// High-level implemented surface summary.
  pub current_implementation: String,
  /// Honest list of still-missing major areas.
  pub not_implemented: String,
}

impl ProjectStatusReport {
  fn current_implementation_items() -> &'static [&'static str] {
    &[
      "architecture, docs, config models",
      "Midnight-backed BN254 fp/fp2/fp6/fp12 arithmetic",
      "minimal G1 add/on-curve checks",
      "narrow G2 affine assign/on-curve/neg",
      "Jacobian projective from_affine/add/double/neg",
      "real optimal-ate Miller-path G2 prep/loop",
      "narrow final exponentiation",
      "narrow multi-pairing product check",
      "the first narrow Groth16 BN254 verifier path",
      "CLI, and sanity-check benches",
    ]
  }

  fn not_implemented_items() -> &'static [&'static str] {
    &[
      "G2 subgroup checks",
      "broad public scalar-multiplication APIs beyond the verifier-only IC path",
      "generalized verifier frameworks",
      "fast always-on end-to-end outer-proof CI coverage",
      "production wrapper verifier circuits",
    ]
  }

  fn join_items(items: &[&str]) -> String {
    items.join(", ")
  }

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
        "the direct outer Halo2/Midnight lane now supports setup, prove, verify, and CLI execution on the canonical outer circuit, but the expensive proving tests remain ignored in the default test lane".to_owned(),
        "there is still no broad wrapper-verifier orchestration, subgroup coverage, or production-hardened artifact ecosystem support".to_owned(),
      ],
    }
  }

  /// Returns a short canonical overview for CLI/about-style reporting.
  #[must_use]
  pub fn overview() -> ProjectOverview {
    ProjectOverview {
      phase_label: "stage 1 / week 5+ (direct outer setup/prove/verify lane landed)".to_owned(),
      purpose: "stage a serious multi-crate codebase for Halo2 wrapper research.".to_owned(),
      current_implementation: format!(
        "{}.",
        Self::join_items(Self::current_implementation_items())
      ),
      not_implemented: format!("{}.", Self::join_items(Self::not_implemented_items())),
    }
  }
}
