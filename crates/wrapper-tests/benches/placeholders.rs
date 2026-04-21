//! Criterion benchmark entry point for workspace placeholder benchmarks.
#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main};
use wrapper_backends as _;
use wrapper_circuits as _;
use wrapper_core as _;
use wrapper_tests as _;

mod ecc;
mod field;
mod pairing;

criterion_group!(
  placeholder_benches,
  field::bench_placeholder_fp,
  ecc::bench_placeholder_ecc,
  pairing::bench_placeholder_pairing
);
criterion_main!(placeholder_benches);
