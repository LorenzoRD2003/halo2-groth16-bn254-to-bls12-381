//! Criterion benchmark entry point for Week 1 primitive benchmarks.
#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main};
use wrapper_backends as _;
use wrapper_core as _;
use wrapper_tests as _;

mod ecc;
mod field;

criterion_group!(
  week1_primitive_benches,
  field::bench_fp_add,
  field::bench_fp_mul,
  ecc::bench_g1_add
);
criterion_main!(week1_primitive_benches);
