//! Backend registry placeholders.

use wrapper_core::{ProofSystemDescriptor, ProofSystemKind};

/// Describes a backend family that may be integrated later.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendDescriptor {
  /// Short backend identifier.
  pub id: &'static str,
  /// Human-readable description.
  pub description: &'static str,
  /// Proof system targeted by the backend.
  pub proof_system: ProofSystemDescriptor,
}

/// Registry of known backend placeholders.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendRegistry {
  entries: Vec<BackendDescriptor>,
}

impl BackendRegistry {
  /// Returns the current backend registry placeholder set.
  #[must_use]
  pub fn scaffold() -> Self {
    Self {
      entries: vec![
        BackendDescriptor {
          id: "placeholder-arkworks",
          description: "Reserved for future arkworks-oriented artifact integration",
          proof_system: ProofSystemDescriptor {
            kind: ProofSystemKind::Groth16Bn254,
            source: "future-arkworks-adapter".to_owned(),
          },
        },
        BackendDescriptor {
          id: "placeholder-midnight",
          description: "Reserved for future Midnight or related ecosystem adapters",
          proof_system: ProofSystemDescriptor {
            kind: ProofSystemKind::Halo2Outer,
            source: "future-midnight-adapter".to_owned(),
          },
        },
      ],
    }
  }

  /// Returns all registered placeholder backends.
  #[must_use]
  pub fn entries(&self) -> &[BackendDescriptor] {
    &self.entries
  }
}
