//! Placeholder ECC benchmarks.

use criterion::{BenchmarkId, Criterion, black_box};

fn placeholder_ecc_walk(points: usize) -> (u64, u64) {
  let mut x = 1_u64;
  let mut y = 2_u64;

  for index in 0..points {
    x = x.wrapping_add((index as u64).rotate_left(5));
    y = y.wrapping_add(x ^ 0xa5a5_a5a5);
  }

  (x, y)
}

/// Benchmarks placeholder ECC-like work.
pub fn bench_placeholder_ecc(criterion: &mut Criterion) {
  let mut group = criterion.benchmark_group("ecc");
  group.bench_with_input(
    BenchmarkId::new("bench_placeholder_ecc", 512),
    &512_usize,
    |bench, points| bench.iter(|| black_box(placeholder_ecc_walk(*points))),
  );
  group.finish();
}
