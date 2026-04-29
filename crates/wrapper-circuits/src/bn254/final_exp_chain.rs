//! Shared BN254 final-exponentiation chain metadata.
//!
//! This module is the single source of truth for the fixed `x`-exponent chain
//! used by `exp_by_neg_x(...)` on both the host/reference side and the
//! circuit-side implementation.

use std::collections::{BTreeSet, HashMap};

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
pub enum Bn254ExpByXWindowSign {
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

/// Cost proxy weights for automated `exp_by_neg_x(...)` chain search.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bn254ExpByXChainSearchWeights {
  /// Proxy cost of one compressed square block indexed by `square_count`.
  ///
  /// Index `0` is unused; valid block lengths start at `1`.
  pub square_block_costs: [u64; 13],
  /// Cost of one positive window multiplication.
  pub positive_mul_cost: u64,
  /// Cost of one negative window multiplication via conjugation.
  pub negative_mul_cost: u64,
  /// Cost charged once per distinct precomputed odd window.
  pub unique_window_cost: u64,
  /// Additional cost charged for the starting window.
  pub start_window_cost: u64,
}

impl Default for Bn254ExpByXChainSearchWeights {
  fn default() -> Self {
    Self::linear(1, 11, 10, 4, 2)
  }
}

impl Bn254ExpByXChainSearchWeights {
  /// Builds a simple linear proxy where one square block of length `n` costs
  /// `n * square_cost`.
  #[must_use]
  pub fn linear(
    square_cost: u64,
    positive_mul_cost: u64,
    negative_mul_cost: u64,
    unique_window_cost: u64,
    start_window_cost: u64,
  ) -> Self {
    let mut square_block_costs = [0_u64; 13];
    let mut index = 1;
    while index < square_block_costs.len() {
      square_block_costs[index] = square_cost * index as u64;
      index += 1;
    }

    Self {
      square_block_costs,
      positive_mul_cost,
      negative_mul_cost,
      unique_window_cost,
      start_window_cost,
    }
  }

  #[must_use]
  /// Returns the proxy cost charged for one compressed square block of the
  /// given length.
  pub const fn square_block_cost(self, square_count: u8) -> u64 {
    self.square_block_costs[square_count as usize]
  }
}

/// One chain step used by the automated search harness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bn254ExpByXSearchStep {
  /// Number of compressed cyclotomic squarings before consuming this window.
  pub square_count: u8,
  /// Whether the consumed window is added or subtracted.
  pub sign: Bn254ExpByXWindowSign,
  /// Absolute odd window value consumed at this step.
  pub window: u64,
}

/// One candidate chain returned by the automated search harness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bn254ExpByXChainCandidate {
  /// Odd starting window value.
  pub start_window: u64,
  /// Signed shift-and-add steps applied after the starting window.
  pub steps: Vec<Bn254ExpByXSearchStep>,
  /// Reconstructed exponent value; must equal `BN254_X_ABS` for valid candidates.
  pub reconstructed_value: u64,
  /// Proxy cost attributed to compressed square blocks.
  pub square_cost_total: u64,
  /// Proxy cost attributed to positive/negative window multiplies.
  pub multiply_cost_total: u64,
  /// Proxy cost attributed to precomputing distinct windows.
  pub precompute_cost_total: u64,
  /// Total proxy score used for ranking.
  pub total_cost: u64,
}

impl Bn254ExpByXChainCandidate {
  /// Returns the distinct odd windows used by this candidate, including the start.
  #[must_use]
  pub fn unique_windows(&self) -> Vec<u64> {
    let mut windows = BTreeSet::new();
    windows.insert(self.start_window);
    for step in &self.steps {
      windows.insert(step.window);
    }
    windows.into_iter().collect()
  }

  /// Returns the schedule rendered in the same style as the retained chain notes.
  #[must_use]
  pub fn schedule_string(&self) -> String {
    let mut parts = vec![self.start_window.to_string()];
    for step in &self.steps {
      let sign = match step.sign {
        Bn254ExpByXWindowSign::Positive => '+',
        Bn254ExpByXWindowSign::Negative => '-',
      };
      parts.push(format!("<<{},{}{}", step.square_count, sign, step.window));
    }
    parts.join(" ")
  }
}

/// Search configuration for automated `exp_by_neg_x(...)` chain exploration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bn254ExpByXChainSearchConfig {
  /// Largest odd window value allowed in the search.
  pub max_window: u64,
  /// Largest square block length considered by one backward step.
  pub max_square_count: u8,
  /// Maximum number of steps after the starting window.
  pub max_steps: usize,
  /// Maximum number of raw candidates to keep before reranking.
  pub max_candidates: usize,
  /// Proxy weights used to score candidates.
  pub weights: Bn254ExpByXChainSearchWeights,
}

impl Default for Bn254ExpByXChainSearchConfig {
  fn default() -> Self {
    Self {
      max_window: 127,
      max_square_count: 12,
      max_steps: 8,
      max_candidates: 256,
      weights: Bn254ExpByXChainSearchWeights::default(),
    }
  }
}

fn score_chain_candidate(
  start_window: u64,
  steps: &[Bn254ExpByXSearchStep],
  weights: Bn254ExpByXChainSearchWeights,
) -> Bn254ExpByXChainCandidate {
  let reconstructed_value = reconstruct_chain_value(start_window, steps);
  let square_cost_total = steps.iter().map(|step| weights.square_block_cost(step.square_count)).sum();
  let multiply_cost_total = steps
    .iter()
    .map(|step| match step.sign {
      Bn254ExpByXWindowSign::Positive => weights.positive_mul_cost,
      Bn254ExpByXWindowSign::Negative => weights.negative_mul_cost,
    })
    .sum();

  let mut unique_windows = BTreeSet::new();
  unique_windows.insert(start_window);
  for step in steps {
    unique_windows.insert(step.window);
  }
  let precompute_cost_total =
    weights.start_window_cost + weights.unique_window_cost * unique_windows.len() as u64;
  let total_cost = square_cost_total + multiply_cost_total + precompute_cost_total;

  Bn254ExpByXChainCandidate {
    start_window,
    steps: steps.to_vec(),
    reconstructed_value,
    square_cost_total,
    multiply_cost_total,
    precompute_cost_total,
    total_cost,
  }
}

fn reconstruct_chain_value(start_window: u64, steps: &[Bn254ExpByXSearchStep]) -> u64 {
  let mut value = start_window;
  for step in steps {
    let shifted = value << step.square_count;
    value = match step.sign {
      Bn254ExpByXWindowSign::Positive => shifted + step.window,
      Bn254ExpByXWindowSign::Negative => shifted - step.window,
    };
  }
  value
}

fn backward_search_candidates(
  current: u64,
  allowed_windows: &[u64],
  config: Bn254ExpByXChainSearchConfig,
  steps_reversed: &mut Vec<Bn254ExpByXSearchStep>,
  best_seen: &mut HashMap<(u64, usize), u64>,
  candidates: &mut Vec<Bn254ExpByXChainCandidate>,
  seen_signatures: &mut BTreeSet<String>,
) {
  if steps_reversed.len() >= config.max_steps {
    return;
  }

  let running_cost: u64 = steps_reversed
    .iter()
    .map(|step| {
      config.weights.square_block_cost(step.square_count) + match step.sign {
          Bn254ExpByXWindowSign::Positive => config.weights.positive_mul_cost,
          Bn254ExpByXWindowSign::Negative => config.weights.negative_mul_cost,
        }
    })
    .sum();

  let state_key = (current, steps_reversed.len());
  if best_seen.get(&state_key).is_some_and(|best| *best <= running_cost) {
    return;
  }
  best_seen.insert(state_key, running_cost);

  for &window in allowed_windows {
    for square_count in 1..=config.max_square_count {
      let shift = 1_u64 << square_count;

      if current > window {
        let delta = current - window;
        if delta % shift == 0 {
          let prev = delta / shift;
          if prev % 2 == 1 && prev > 0 {
            steps_reversed.push(Bn254ExpByXSearchStep {
              square_count,
              sign: Bn254ExpByXWindowSign::Positive,
              window,
            });
            maybe_record_candidate(
              prev,
              steps_reversed,
              allowed_windows,
              config,
              candidates,
              seen_signatures,
            );
            backward_search_candidates(
              prev,
              allowed_windows,
              config,
              steps_reversed,
              best_seen,
              candidates,
              seen_signatures,
            );
            steps_reversed.pop();
          }
        }
      }

      let sum = current + window;
      if sum % shift == 0 {
        let prev = sum / shift;
        if prev % 2 == 1 && prev > 0 {
          steps_reversed.push(Bn254ExpByXSearchStep {
            square_count,
            sign: Bn254ExpByXWindowSign::Negative,
            window,
          });
          maybe_record_candidate(
            prev,
            steps_reversed,
            allowed_windows,
            config,
            candidates,
            seen_signatures,
          );
          backward_search_candidates(
            prev,
            allowed_windows,
            config,
            steps_reversed,
            best_seen,
            candidates,
            seen_signatures,
          );
          steps_reversed.pop();
        }
      }
    }
  }
}

fn maybe_record_candidate(
  start_window: u64,
  steps_reversed: &[Bn254ExpByXSearchStep],
  allowed_windows: &[u64],
  config: Bn254ExpByXChainSearchConfig,
  candidates: &mut Vec<Bn254ExpByXChainCandidate>,
  seen_signatures: &mut BTreeSet<String>,
) {
  if !allowed_windows.contains(&start_window) {
    return;
  }

  let forward_steps: Vec<_> = steps_reversed.iter().rev().copied().collect();
  let candidate = score_chain_candidate(start_window, &forward_steps, config.weights);
  if candidate.reconstructed_value != BN254_X_ABS {
    return;
  }

  let signature = format!("{}|{}", start_window, candidate.schedule_string());
  if seen_signatures.insert(signature) {
    candidates.push(candidate);
  }
}

/// Returns the current retained chain scored under the given proxy weights.
#[must_use]
pub fn retained_bn254_exp_by_x_chain_candidate(
  weights: Bn254ExpByXChainSearchWeights,
) -> Bn254ExpByXChainCandidate {
  let steps = BN254_EXP_BY_X_CHAIN_STEPS
    .iter()
    .map(|step| Bn254ExpByXSearchStep {
      square_count: step.square_count,
      sign: step.sign,
      window: match step.window {
        Bn254ExpByXWindow::X17 => 17,
        Bn254ExpByXWindow::X35 => 35,
        Bn254ExpByXWindow::X37 => 37,
        Bn254ExpByXWindow::X79 => 79,
        Bn254ExpByXWindow::X83 => 83,
        Bn254ExpByXWindow::X101 => 101,
        Bn254ExpByXWindow::X105 => 105,
      },
    })
    .collect::<Vec<_>>();

  score_chain_candidate(35, &steps, weights)
}

/// Searches for low-cost candidate chains that reconstruct `BN254_X_ABS`
/// under the provided proxy configuration.
#[must_use]
pub fn search_bn254_exp_by_x_candidates(
  config: Bn254ExpByXChainSearchConfig,
) -> Vec<Bn254ExpByXChainCandidate> {
  let allowed_windows: Vec<u64> = (1..=config.max_window).filter(|value| value % 2 == 1).collect();
  search_bn254_exp_by_x_candidates_with_windows(config, &allowed_windows)
}

/// Searches for low-cost candidate chains using one explicit set of allowed odd
/// windows.
#[must_use]
pub fn search_bn254_exp_by_x_candidates_with_windows(
  config: Bn254ExpByXChainSearchConfig,
  allowed_windows: &[u64],
) -> Vec<Bn254ExpByXChainCandidate> {
  let mut allowed_windows = allowed_windows
    .iter()
    .copied()
    .filter(|value| *value > 0 && *value % 2 == 1)
    .collect::<Vec<_>>();
  allowed_windows.sort_unstable();
  allowed_windows.dedup();
  let mut steps_reversed = Vec::new();
  let mut best_seen = HashMap::new();
  let mut candidates = Vec::new();
  let mut seen_signatures = BTreeSet::new();

  backward_search_candidates(
    BN254_X_ABS,
    &allowed_windows,
    config,
    &mut steps_reversed,
    &mut best_seen,
    &mut candidates,
    &mut seen_signatures,
  );

  candidates.sort_by_key(|candidate| {
    (
      candidate.total_cost,
      candidate.steps.len(),
      candidate.unique_windows().len(),
      candidate.start_window,
      candidate.schedule_string(),
    )
  });
  candidates.truncate(config.max_candidates);
  candidates
}

#[cfg(test)]
mod tests {
  use super::{
    BN254_EXP_BY_X_CHAIN_START, BN254_EXP_BY_X_CHAIN_STEPS, BN254_X_ABS,
    Bn254ExpByXChainSearchConfig, Bn254ExpByXChainSearchWeights,
    retained_bn254_exp_by_x_chain_candidate, search_bn254_exp_by_x_candidates,
    search_bn254_exp_by_x_candidates_with_windows,
  };

  #[test]
  fn exp_by_x_chain_reconstructs_bn254_parameter() {
    let mut value = BN254_EXP_BY_X_CHAIN_START.value();

    for step in BN254_EXP_BY_X_CHAIN_STEPS {
      value = step.sign.apply(value << step.square_count, step.window.value());
    }

    assert_eq!(value, BN254_X_ABS);
  }

  #[test]
  fn retained_candidate_scores_and_reconstructs_parameter() {
    let candidate =
      retained_bn254_exp_by_x_chain_candidate(Bn254ExpByXChainSearchWeights::default());
    assert_eq!(candidate.reconstructed_value, BN254_X_ABS);
    assert_eq!(candidate.start_window, 35);
    assert!(!candidate.steps.is_empty());
  }

  #[test]
  fn automated_search_returns_candidates_for_default_config() {
    let candidates = search_bn254_exp_by_x_candidates(Bn254ExpByXChainSearchConfig::default());
    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|candidate| candidate.reconstructed_value == BN254_X_ABS));
  }

  #[test]
  fn automated_search_returns_candidates_for_retained_window_set() {
    let candidates = search_bn254_exp_by_x_candidates_with_windows(
      Bn254ExpByXChainSearchConfig::default(),
      &[17, 35, 37, 79, 83, 101, 105],
    );
    assert!(!candidates.is_empty());
    assert!(candidates.iter().all(|candidate| candidate.reconstructed_value == BN254_X_ABS));
  }
}
