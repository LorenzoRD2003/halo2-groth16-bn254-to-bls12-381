//! Placeholder field benchmarks.

use criterion::{BenchmarkId, Criterion, black_box};

fn placeholder_field_fold(iterations: u64) -> u64 {
  (0..iterations)
    .fold(0_u64, |accumulator, value| accumulator.wrapping_add(value.rotate_left(7) ^ 0x9e37_79b9))
}

/// Benchmarks placeholder field-like arithmetic work.
pub fn bench_placeholder_fp(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("field");
  group.bench_with_input(
    BenchmarkId::new("bench_placeholder_fp", 1_024),
    &1_024_u64,
    |bench, iterations| bench.iter(|| black_box(placeholder_field_fold(*iterations))),
  );
  group.finish();
}
