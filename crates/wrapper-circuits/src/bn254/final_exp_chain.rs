//! Shared BN254 final-exponentiation chain metadata.
//!
//! This module is the single source of truth for the fixed `x`-exponent chain
//! used by `exp_by_neg_x(...)` on both the host/reference side and the
//! circuit-side implementation.

/// Absolute value of the BN parameter `x` used by the BN254 final exponentiation.
pub(crate) const BN254_X_ABS: u64 = 4_965_661_367_192_848_881;

/// One precomputed odd window used by the fixed `exp_by_neg_x(...)` chain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Bn254ExpByXWindow {
  X17,
  X25,
  X29,
  X39,
  X41,
  X43,
  X49,
}

impl Bn254ExpByXWindow {
  /// Returns the odd integer represented by this precomputed window.
  #[must_use]
  #[cfg(test)]
  pub(crate) const fn value(self) -> u64 {
    match self {
      Self::X17 => 17,
      Self::X25 => 25,
      Self::X29 => 29,
      Self::X39 => 39,
      Self::X41 => 41,
      Self::X43 => 43,
      Self::X49 => 49,
    }
  }
}

/// Starting window of the fixed `exp_by_neg_x(...)` chain.
pub(crate) const BN254_EXP_BY_X_CHAIN_START: Bn254ExpByXWindow = Bn254ExpByXWindow::X17;

/// Shift-and-add plan for the fixed `exp_by_neg_x(...)` chain.
///
/// Starting from `17`, each step means:
///
/// `acc = (acc << square_count) + next_window`
pub(crate) const BN254_EXP_BY_X_CHAIN_STEPS: &[(u8, Bn254ExpByXWindow)] = &[
  (7, Bn254ExpByXWindow::X29),
  (7, Bn254ExpByXWindow::X25),
  (8, Bn254ExpByXWindow::X43),
  (6, Bn254ExpByXWindow::X17),
  (8, Bn254ExpByXWindow::X41),
  (6, Bn254ExpByXWindow::X41),
  (10, Bn254ExpByXWindow::X39),
  (6, Bn254ExpByXWindow::X49),
];

#[cfg(test)]
mod tests {
  use super::{BN254_EXP_BY_X_CHAIN_START, BN254_EXP_BY_X_CHAIN_STEPS, BN254_X_ABS};

  #[test]
  fn exp_by_x_chain_reconstructs_bn254_parameter() {
    let mut value = BN254_EXP_BY_X_CHAIN_START.value();

    for (square_count, window) in BN254_EXP_BY_X_CHAIN_STEPS {
      value = (value << square_count) + window.value();
    }

    assert_eq!(value, BN254_X_ABS);
  }
}
