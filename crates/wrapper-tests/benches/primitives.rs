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
  field::bench_fp6_add,
  field::bench_fp6_mul,
  field::bench_fp6_square,
  field::bench_fp12_add,
  field::bench_fp12_mul,
  field::bench_fp12_square,
  ecc::bench_g1_add,
  ecc::bench_g2_on_curve,
  ecc::bench_g2_neg,
  ecc::bench_g2_proj_from_affine,
  ecc::bench_g2_proj_double,
  ecc::bench_g2_proj_add,
  ecc::bench_g2_double_with_line,
  ecc::bench_g2_mixed_add_with_line,
  ecc::bench_miller_accumulator_square,
  ecc::bench_miller_accumulator_mul_by_line,
  ecc::bench_miller_accumulator_mul_by_line_sparse,
  ecc::bench_miller_loop_narrow,
  ecc::bench_final_exponentiation,
  ecc::bench_pairing_check
);
criterion_main!(primitive_benches);
