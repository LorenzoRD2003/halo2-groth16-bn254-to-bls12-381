//! Placeholder pairing benchmarks.

use criterion::{BenchmarkId, Criterion, black_box};

fn placeholder_pairing_mix(rounds: usize) -> [u64; 4] {
  let mut state = [3_u64, 5_u64, 8_u64, 13_u64];

  for round in 0..rounds {
    let shift: u32 = u32::try_from(round % 31).unwrap_or_default();
    let rotated = state[round % state.len()].rotate_left(shift);
    state[0] = state[0].wrapping_add(rotated);
    state[1] ^= state[0].wrapping_mul(17);
    state[2] = state[2].wrapping_add(state[1] ^ 0xfeed_face);
    state[3] = state[3].wrapping_add(state[2].rotate_left(11));
  }

  state
}

/// Benchmarks placeholder pairing-like work.
pub fn bench_placeholder_pairing(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("pairing");
  group.bench_with_input(
    BenchmarkId::new("bench_placeholder_pairing", 256),
    &256_usize,
    |bench, rounds| bench.iter(|| black_box(placeholder_pairing_mix(*rounds))),
  );
  group.finish();
}
