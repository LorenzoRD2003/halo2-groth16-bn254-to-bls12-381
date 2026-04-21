//! Criterion benchmark entry point for the current primitive benchmarks.
#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main};
use wrapper_backends as _;
use wrapper_core as _;
use wrapper_tests as _;

mod ecc;
mod field;

criterion_group!(
  primitive_benches,
  field::bench_fp_add,
  field::bench_fp_mul,
  field::bench_fp2_add,
  field::bench_fp2_mul,
  field::bench_fp2_square,
  ecc::bench_g1_add
);
criterion_main!(primitive_benches);
