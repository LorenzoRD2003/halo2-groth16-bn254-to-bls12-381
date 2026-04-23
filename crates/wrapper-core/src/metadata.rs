//! Proof and verification-key metadata abstractions.

use serde::{Deserialize, Serialize};

/// Supported proof-system families for normalized metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProofSystemKind {
  /// Groth16 over BN254 is a future target of this repository.
  Groth16Bn254,
  /// Groth16 over BLS12-381 is the current outer-proof target for migration experiments.
  Groth16Bls12_381,
  /// Placeholder for future outer proof systems.
  Halo2Outer,
}

/// Proof-system descriptor independent of any concrete backend crate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProofSystemDescriptor {
  /// Proof-system kind.
  pub kind: ProofSystemKind,
  /// Human-readable provenance string.
  pub source: String,
}

/// Normalized proof metadata without cryptographic claims.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NormalizedProofArtifact {
  /// Logical identifier for the artifact.
  pub identifier: String,
  /// Associated proof-system descriptor.
  pub proof_system: ProofSystemDescriptor,
  /// Optional notes from parsing or ingestion.
  pub notes: Vec<String>,
}

/// Normalized verification-key metadata without cryptographic material.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NormalizedVerificationKey {
  /// Logical identifier for the verification key.
  pub identifier: String,
  /// Associated proof-system descriptor.
  pub proof_system: ProofSystemDescriptor,
  /// Whether material has only been declared, not loaded.
  pub declared_only: bool,
}
