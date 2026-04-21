# Benchmarking

This repository uses Criterion for benchmark scaffolding.

Current status:

- benchmark structure exists
- benchmark commands work
- benchmark logic is placeholder-only

No current benchmark should be interpreted as a cryptographic performance claim.

## Running Benchmarks

Run all benchmarks with:

```bash
cargo bench
```

The current benchmark target lives in `crates/wrapper-tests/benches/placeholders.rs`.

## Current Structure

Benchmarks are grouped by future implementation area:

- `crates/wrapper-tests/benches/field/`
- `crates/wrapper-tests/benches/ecc/`
- `crates/wrapper-tests/benches/pairing/`

This keeps the future benchmark layout aligned with the intended cryptographic workstreams without forcing those implementations to exist yet.

## Adding a New Benchmark

1. Put the benchmark helper or module in the appropriate directory under `crates/wrapper-tests/benches/`.
2. Register the benchmark function from `placeholders.rs` through `criterion_group!`.
3. Keep the benchmark logic honest and explicit about whether it measures a placeholder, parser path, layout calculation, or real cryptographic code.
4. If the benchmark represents a new category of work, update this document and `wrapper-cli bench-info`.

## Naming Convention

Future benchmark names should follow:

```text
bench_<module>_<operation>
```

Examples:

- `bench_field_range_check`
- `bench_ecc_point_add`
- `bench_pairing_miller_loop`

The current bootstrap placeholders are intentionally generic and exist only to validate the benchmarking workflow.

## Metrics That Will Matter Later

Once real cryptographic code exists, the most important metrics are expected to include:

- constraints or rows
- proving time
- memory usage

Additional metrics may be added later if circuit shape, witness generation, or backend serialization become important bottlenecks.

## Warning

All current benchmarks are placeholders only. They do not measure field arithmetic, ECC, pairings, Groth16 verification, or any real Halo2 cryptographic circuit logic.

