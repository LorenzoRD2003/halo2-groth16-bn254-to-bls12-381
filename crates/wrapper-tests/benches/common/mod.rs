//! Shared benchmark helpers for Criterion-backed workspace benches.

use std::{
  sync::Once,
  time::{Duration, Instant},
};

use criterion::{BenchmarkId, Criterion};

static BENCH_TIMING_HEADER: Once = Once::new();

fn print_benchmark_timing_header() {
  BENCH_TIMING_HEADER.call_once(|| {
    eprintln!("kind\tid\telapsed_ms");
  });
}

fn time_one_run(run: fn()) -> Duration {
  let started_at = Instant::now();
  run();
  started_at.elapsed()
}

/// Runs one deterministic preflight sample, prints its elapsed wall-clock time,
/// and then registers the Criterion benchmark.
pub fn bench_verified_sample(
  criterion: &mut Criterion,
  group_name: &str,
  bench_name: &str,
  run: fn(),
) {
  let elapsed = time_one_run(run);
  print_benchmark_timing_header();
  eprintln!("benchmark\t{bench_name}\t{}", elapsed.as_millis());

  let mut group = criterion.benchmark_group(group_name);
  group.bench_with_input(BenchmarkId::new(bench_name, 1), &1_u8, |bench, _| bench.iter(run));
  group.finish();
}
