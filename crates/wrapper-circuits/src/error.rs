//! Error types for circuit-facing arithmetic layers.

use thiserror::Error;

/// Errors raised by the Week 1 BN254 foundation layer.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum CircuitError {
  /// The provided coordinates do not satisfy the BN254 G1 curve equation.
  #[error("invalid bn254 g1 point: coordinates are not on the curve")]
  InvalidBn254Point,
}
