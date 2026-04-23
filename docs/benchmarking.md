# Benchmarking

This repository uses Criterion for benchmark scaffolding.

For reproducible layout-cost baselines on the current Groth16 BN254 verifier
slice, use `wrapper-cli profile-layout` as described in `docs/profiling.md`.
Criterion remains the home for benchmark hooks; `profile-layout` is the narrow
TSV-reporting path for optimization baselines and before/after diffs.

Current status:

- benchmark structure exists
- benchmark commands work
- the current primitive layer includes small Midnight-backed circuits for field add, field mul, fp2 add, fp2 mul, fp2 square, fp6 add, fp6 mul, fp6 square, fp12 add, fp12 mul, fp12 square, G1 add, G2 on-curve, G2 neg, G2 projective from-affine embedding, G2 projective doubling, G2 projective addition, G2 doubling-with-line, G2 mixed-add-with-line, Miller-accumulator square, Miller-accumulator mul-by-line, Miller-accumulator sparse mul-by-line, a narrow Miller-loop slice, narrow final exponentiation, and a narrow pairing-check slice
- benchmark coverage is still intentionally narrow

No current benchmark should be interpreted as a production cryptographic performance claim.

## Running Benchmarks

Run all benchmarks with:

```bash
cargo bench
```

The current benchmark target lives in `crates/wrapper-tests/benches/primitives.rs`.

## Profiling vs Benchmarks

Use:

- `cargo bench` for Criterion benchmark hooks
- `cargo run -p wrapper-cli -- profile-layout ...` for reproducible layout
  metric capture on the current Groth16 slice

The profiling workflow is documented in `docs/profiling.md` and is the
preferred baseline path for optimization work on:

- total Groth16 verifier layout cost
- pairing-term scaling
- public-input scaling
- existing pairing-core block costs

For final-exponentiation-specific audit and next-step optimization planning, see
`docs/final-exponentiation-audit.md`.

## Current Structure

Benchmarks are grouped by future implementation area:

- `crates/wrapper-tests/benches/field/`
- `crates/wrapper-tests/benches/ecc/`

This keeps the benchmark layout aligned with the intended cryptographic workstreams without forcing later-stage implementations to exist yet.

The benchmark metadata shown by `wrapper-cli bench-info` is now derived from the canonical primitive registry in `crates/wrapper-circuits/src/planning.rs`, so bench names and measured primitive labels should be updated there first.

## Adding a New Benchmark

1. Put the benchmark helper or module in the appropriate directory under `crates/wrapper-tests/benches/`.
2. Register the benchmark function from `primitives.rs` through `criterion_group!`.
3. Add or update the canonical primitive metadata in `crates/wrapper-circuits/src/planning.rs` so `wrapper-cli bench-info` and measured primitive reporting stay aligned.
4. Keep the benchmark logic honest and explicit about whether it measures a placeholder, parser path, layout calculation, or real cryptographic code.
5. If the benchmark represents a new category of work, update this document as well.

## Naming Convention

Future benchmark names should follow:

```text
bench_<module>_<operation>
```

Examples:

- `bench_field_range_check`
- `bench_ecc_point_add`
- `bench_pairing_miller_loop`

Current benchmark entry points are:

- `bench_fp_add`
- `bench_fp_mul`
- `bench_fp2_add`
- `bench_fp2_mul`
- `bench_fp2_square`
- `bench_fp6_add`
- `bench_fp6_mul`
- `bench_fp6_square`
- `bench_fp12_add`
- `bench_fp12_mul`
- `bench_fp12_square`
- `bench_g1_add`
- `bench_g2_on_curve`
- `bench_g2_neg`
- `bench_g2_proj_from_affine`
- `bench_g2_proj_double`
- `bench_g2_proj_add`
- `bench_g2_double_with_line`
- `bench_g2_mixed_add_with_line`
- `bench_miller_accumulator_square`
- `bench_miller_accumulator_mul_by_line`
- `bench_miller_accumulator_mul_by_line_sparse`
- `bench_miller_loop_narrow`
- `bench_final_exponentiation`
- `bench_pairing_check`

## Metrics That Will Matter Later

Once real cryptographic code exists, the most important metrics are expected to include:

- constraints or rows
- proving time
- memory usage

Additional metrics may be added later if circuit shape, witness generation, or backend serialization become important bottlenecks.

## Warning

Current benchmarks exercise small Midnight-backed BN254 primitive circuits. The Miller-loop benchmarks cover the current narrow accumulation slice over extracted lines, `bench_final_exponentiation` covers the current narrow final-exponentiation slice on top of an Fp12 Miller output, and `bench_pairing_check` covers the narrow multi-pairing product-check slice with one shared final exponentiation. `bench_miller_accumulator_mul_by_line` is retained as the generic baseline path, while `bench_miller_accumulator_mul_by_line_sparse` measures the optimized sparse-specialized line-consumption path used by the public accumulator API. These do not measure subgroup checks, scalar multiplication, a broad verifier-facing pairing API, Groth16 verification, or a production wrapper circuit.
