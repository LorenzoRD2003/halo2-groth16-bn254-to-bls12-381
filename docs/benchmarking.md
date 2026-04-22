# Benchmarking

This repository uses Criterion for benchmark scaffolding.

Current status:

- benchmark structure exists
- benchmark commands work
- the current primitive layer includes small Midnight-backed circuits for field add, field mul, fp2 add, fp2 mul, fp2 square, fp6 add, fp6 mul, fp6 square, fp12 add, fp12 mul, fp12 square, G1 add, G2 on-curve, G2 neg, G2 projective from-affine embedding, G2 projective doubling, G2 projective addition, G2 doubling-with-line, G2 mixed-add-with-line, Miller-accumulator square, Miller-accumulator mul-by-line, and a narrow Miller-loop slice
- benchmark coverage is still intentionally narrow

No current benchmark should be interpreted as a production cryptographic performance claim.

## Running Benchmarks

Run all benchmarks with:

```bash
cargo bench
```

The current benchmark target lives in `crates/wrapper-tests/benches/primitives.rs`.

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
- `bench_miller_loop_narrow`

## Metrics That Will Matter Later

Once real cryptographic code exists, the most important metrics are expected to include:

- constraints or rows
- proving time
- memory usage

Additional metrics may be added later if circuit shape, witness generation, or backend serialization become important bottlenecks.

## Warning

Current benchmarks exercise small Midnight-backed BN254 primitive circuits. The Miller-loop benchmarks only cover the current narrow accumulation slice over extracted lines. They do not measure subgroup checks, scalar multiplication, full pairings, final exponentiation, Groth16 verification, or a production wrapper circuit.
