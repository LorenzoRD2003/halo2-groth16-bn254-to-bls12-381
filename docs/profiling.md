# Profiling The Current Groth16 Slice

This repository now includes a minimal layout-profiling workflow for the
current narrow Groth16 BN254 verifier slice.

The goal is reproducible measurement for optimization work, not a new
benchmarking framework.

## What Was Added

- a `wrapper-cli profile-layout` command
- deterministic Groth16 measurement helpers inside `wrapper-circuits`
- stable TSV output that is easy to redirect, diff, and compare across PRs

The new workflow reuses the existing layout-cost path based on
`measure_layout(...)` and the current Halo2/Midnight circuit model.

## What It Measures

The profiling command emits layout metrics for four measurement families.

### 1. `groth16`

These rows cover the current end-to-end verifier slice and its most relevant
Groth16-specific blocks:

- `groth16_fixture_verifier_total`
  Measures the canonical verifier circuit on the committed `circom` / `snarkjs`
  fixture.
- `groth16_fixture_vk_x_accumulator`
  Measures the verifier-side `vk_x` accumulation block on the same fixture.
- `groth16_pairing_check_proxy_4_terms`
  Measures an isolated 4-term pairing-check proxy circuit that matches the
  current Groth16 term count.

### 2. `pairing-terms`

These rows isolate how the narrow pairing-check circuit scales as the number of
pairing terms grows:

- `1`
- `2`
- `3`
- `4`

This is intentionally term-count profiling, not a generalized verifier API.

### 3. `public-inputs`

These rows isolate how the current `vk_x` accumulation path scales as the
number of public inputs grows:

- `1`
- `2`
- `4`
- `8`
- `16`

The VK shape is synthetic but deterministic and stays close to the current
verifier-side accumulation path.

### 4. `blocks`

These rows report the existing narrow pairing-core block measurements that were
already available:

- Miller loop
- final exponentiation easy part
- final exponentiation hard part
- final exponentiation
- pairing check primitive sample

## What It Does Not Measure Yet

This workflow does not currently measure:

- prover runtime
- witness generation runtime
- memory usage
- subgroup checks
- generalized verifier orchestration
- production backend performance
- host-side microbenchmarks

Layout / constraint cost is still the primary signal.

## Output Format

The command prints TSV with a stable header:

```text
family	id	label	term_count	public_input_count	rows	column_queries	k	table_rows	max_degree	advice_columns	fixed_columns	lookups	permutations	point_sets
```

This is designed to be:

- readable in the terminal
- redirectable to a file
- easy to diff before and after an optimization PR

## Commands

Run the full profiling set:

```bash
cargo run -p wrapper-cli -- profile-layout
```

Run only the Groth16 verifier-focused rows:

```bash
cargo run -p wrapper-cli -- profile-layout --family groth16
```

Run only the pairing-term scaling rows:

```bash
cargo run -p wrapper-cli -- profile-layout --family pairing-terms
```

Run only the public-input scaling rows:

```bash
cargo run -p wrapper-cli -- profile-layout --family public-inputs
```

Run only the already-available pairing-core block rows:

```bash
cargo run -p wrapper-cli -- profile-layout --family blocks
```

Save a baseline for later comparison:

```bash
cargo run -p wrapper-cli -- profile-layout > /tmp/groth16-layout-profile.tsv
```

The `groth16`, `pairing-terms`, and `all` modes can take noticeably longer than
`blocks` or `public-inputs`, because they model large pairing-backed circuits.
That is expected: these commands are meant for reproducible baseline capture,
not for tight edit-run loops.

Important workflow note:

- wait for the command to exit before inspecting or diffing the TSV
- if you open the output file while a heavy mode is still running, the file may
  look empty or incomplete simply because the command has not finished yet

## Interpretation Notes

- `groth16_fixture_verifier_total` is the closest current measurement of the
  committed verifier slice end-to-end.
- `groth16_pairing_check_proxy_4_terms` is intentionally a term-count proxy for
  the Groth16 reduction, not a second semantic verifier implementation.
- `public-inputs` isolates the current accumulator path only; it does not
  remeasure the full verifier for every input count.
- `blocks` rows are useful when optimization work targets Miller loop or final
  exponentiation directly.
- for final-exponentiation-specific decomposition, operation counts, and
  follow-up targets, read `docs/final-exponentiation-audit.md`

## Suggested Baseline Workflow

Before starting optimization work:

1. capture `profile-layout` output on the main branch
2. save the TSV artifact
3. rerun the same command after each optimization branch
4. diff the rows that correspond to the block you changed

This keeps optimization work grounded in hard measurements instead of intuition.
