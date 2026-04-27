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
  X35,
  X37,
  X79,
  X83,
  X101,
  X105,
}

impl Bn254ExpByXWindow {
  /// Returns the odd integer represented by this precomputed window.
  #[must_use]
  #[cfg(test)]
  pub(crate) const fn value(self) -> u64 {
    match self {
      Self::X17 => 17,
      Self::X35 => 35,
      Self::X37 => 37,
      Self::X79 => 79,
      Self::X83 => 83,
      Self::X101 => 101,
      Self::X105 => 105,
    }
  }
}

/// One signed step in the fixed `exp_by_neg_x(...)` chain.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Bn254ExpByXChainStep {
  /// Number of cyclotomic squarings to apply before consuming the next window.
  pub square_count: u8,
  /// Whether the consumed window is added or subtracted.
  pub sign: Bn254ExpByXWindowSign,
  /// Window consumed after the squaring block.
  pub window: Bn254ExpByXWindow,
}

/// Sign of one consumed `exp_by_neg_x(...)` window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Bn254ExpByXWindowSign {
  Positive,
  Negative,
}

impl Bn254ExpByXWindowSign {
  #[must_use]
  #[cfg(test)]
  pub(crate) const fn apply(self, accumulator: u64, window: u64) -> u64 {
    match self {
      Self::Positive => accumulator + window,
      Self::Negative => accumulator - window,
    }
  }
}

/// Starting window of the fixed `exp_by_neg_x(...)` chain.
pub(crate) const BN254_EXP_BY_X_CHAIN_START: Bn254ExpByXWindow = Bn254ExpByXWindow::X35;

/// Shift-and-add plan for the fixed `exp_by_neg_x(...)` chain.
///
/// Starting from `35`, each step means:
///
/// `acc = (acc << square_count) +/- next_window`
pub(crate) const BN254_EXP_BY_X_CHAIN_STEPS: &[Bn254ExpByXChainStep] = &[
  Bn254ExpByXChainStep {
    square_count: 6,
    sign: Bn254ExpByXWindowSign::Negative,
    window: Bn254ExpByXWindow::X35,
  },
  Bn254ExpByXChainStep {
    square_count: 9,
    sign: Bn254ExpByXWindowSign::Positive,
    window: Bn254ExpByXWindow::X101,
  },
  Bn254ExpByXChainStep {
    square_count: 8,
    sign: Bn254ExpByXWindowSign::Negative,
    window: Bn254ExpByXWindow::X83,
  },
  Bn254ExpByXChainStep {
    square_count: 9,
    sign: Bn254ExpByXWindowSign::Positive,
    window: Bn254ExpByXWindow::X37,
  },
  Bn254ExpByXChainStep {
    square_count: 9,
    sign: Bn254ExpByXWindowSign::Positive,
    window: Bn254ExpByXWindow::X105,
  },
  Bn254ExpByXChainStep {
    square_count: 11,
    sign: Bn254ExpByXWindowSign::Positive,
    window: Bn254ExpByXWindow::X79,
  },
  Bn254ExpByXChainStep {
    square_count: 5,
    sign: Bn254ExpByXWindowSign::Positive,
    window: Bn254ExpByXWindow::X17,
  },
];

#[cfg(test)]
mod tests {
  use super::{BN254_EXP_BY_X_CHAIN_START, BN254_EXP_BY_X_CHAIN_STEPS, BN254_X_ABS};

  #[test]
  fn exp_by_x_chain_reconstructs_bn254_parameter() {
    let mut value = BN254_EXP_BY_X_CHAIN_START.value();

    for step in BN254_EXP_BY_X_CHAIN_STEPS {
      value = step.sign.apply(value << step.square_count, step.window.value());
    }

    assert_eq!(value, BN254_X_ABS);
  }
}
