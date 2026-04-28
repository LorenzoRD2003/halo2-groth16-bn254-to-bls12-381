# Profiling The Current Groth16 Slice

This repository now includes a minimal layout-profiling workflow for the
current narrow Groth16 BN254 verifier slice.

The goal is reproducible measurement for optimization work, not a new
benchmarking framework.

For the current prioritized list of local optimization opportunities backed by
existing Midnight primitives, see `docs/midnight-local-optimization-notes.md`.

## What Was Added

- a `wrapper-cli profile-layout` command
- deterministic Groth16 measurement helpers inside `wrapper-circuits`
- stable TSV output that is easy to redirect, diff, and compare across PRs

The new workflow reuses the existing layout-cost path based on
`measure_layout(...)` and the current Halo2/Midnight circuit model.

## What It Measures

The profiling command emits layout metrics for five measurement families.

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

The current Groth16 verifier path now precomputes Miller-step line
coefficients off-circuit for constant verifier-key G2 terms:

- `beta_g2`
- `gamma_g2`
- `delta_g2`

This is valid because those G2 points are fixed verifier-key data, not proof
witnesses. The tradeoff is a larger prepared verifier-key representation in host
memory and orchestration code in exchange for lower circuit cost.

### 2. `pairing-terms`

These rows isolate how the narrow pairing-check circuit scales as the number of
pairing terms grows:

- `1`
- `2`
- `3`
- `4`

This is intentionally term-count profiling, not a generalized verifier API.
The current profile models one variable proof-like G2 term and the remaining
terms as prepared constant verifier-key-style G2 terms, which matches the
current Groth16 verifier shape more closely than an all-variable proxy.

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
- pairing check groth16-style (1 variable + 3 prepared)
- pairing check primitive sample

### 5. `outer`

These rows cover the canonical direct outer-wrapper lane on the two committed
Groth16 fixtures and on both implemented Midnight host lanes:

- `circom_multiplier2` on BN254 host
- `circom_multiplier2` on BLS12-381 host
- `semaphore` on BN254 host
- `semaphore` on BLS12-381 host

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
family	id	label	term_count	public_input_count	parse_elapsed_ms	package_elapsed_ms	build_circuit_elapsed_ms	build_elapsed_ms	layout_elapsed_ms	elapsed_ms	rows	column_queries	k	table_rows	max_degree	advice_columns	fixed_columns	lookups	permutations	point_sets
```

This is designed to be:

- readable in the terminal
- redirectable to a file
- easy to diff before and after an optimization PR
- explicit about the wall-clock time spent producing each row
- split for `outer` rows between circuit build/adaptation time and layout-modeling time

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

Run only the direct outer-wrapper rows:

```bash
cargo run -p wrapper-cli -- profile-layout --family outer
```

Run only the already-available pairing-core block rows:

```bash
cargo run -p wrapper-cli -- profile-layout --family blocks
```

Save a baseline for later comparison:

```bash
cargo run -p wrapper-cli -- profile-layout > /tmp/groth16-layout-profile.tsv
```

The `groth16`, `outer`, `pairing-terms`, and `all` modes can take noticeably
longer than `blocks` or `public-inputs`, because they model large pairing-backed
circuits.
That is expected: these commands are meant for reproducible baseline capture,
not for tight edit-run loops.

Important workflow note:

- wait for the command to exit before inspecting or diffing the TSV
- if you open the output file while a heavy mode is still running, the file may
  look empty or incomplete simply because the command has not finished yet

## Interpretation Notes

- `groth16_fixture_verifier_total` is the closest current measurement of the
  committed verifier slice end-to-end.
- `outer_wrapper_circom_multiplier2_end_to_end_bn254_host` and
  `outer_wrapper_circom_multiplier2_end_to_end_bls12_381_host` are the direct
  end-to-end baselines for the committed `circom_multiplier2` fixture across
  both Midnight host lanes.
- `outer_wrapper_semaphore_end_to_end_bn254_host` and
  `outer_wrapper_semaphore_end_to_end_bls12_381_host` are the matching direct
  end-to-end baselines for the committed Semaphore fixture across both Midnight
  host lanes.
- `groth16_pairing_check_proxy_4_terms` is intentionally a term-count proxy for
  the Groth16 reduction, not a second semantic verifier implementation.
- `public-inputs` isolates the current accumulator path only; it does not
  remeasure the full verifier for every input count.
- `blocks` rows are useful when optimization work targets Miller loop or final
  exponentiation directly.
- for local follow-up targets that specifically leverage `midnight-circuits`
  primitives such as `mul_by_constant`, read
  `docs/midnight-local-optimization-notes.md`
- treat `linear_combination(...)` as a measured hypothesis, not as a presumed
  optimization:
  the April 27, 2026 foreign-field pass regressed the retained baseline and was
  reverted, so future local tower rewrites should compare against the current
  `mul_by_constant(...)` path rather than assuming affine-looking formulas are
  cheaper
- treat `add_constant(...)` more narrowly:
  the retained April 27, 2026 win only improved local G2 / Miller-prep metrics
  by folding the fixed BN254 twist coefficient into `assert_on_curve(...)`;
  it did not change the `blocks` rows for Miller loop, final exponentiation, or
  pairing check
- treat `select` / `is_equal*` / `is_zero` similarly as measured control-flow
  hypotheses:
  the April 27, 2026 attempt to replace the manual GT identity check with
  composite boolean equality helpers was row-neutral and was not retained as an
  optimization
- for `exp_by_neg_x(...)`, the retained direction is chain-level rather than
  formula-local:
  the April 27, 2026 signed-window replacement improved
  `bn254_final_exponentiation_hard_part` from `574112` to `561254` rows and
  `bn254_pairing_check_sample_2_terms` from `1682524` to `1669666`
- the next retained step after that was compressed cyclotomic squaring inside
  the repeated square blocks of `exp_by_neg_x(...)`, which improved
  `bn254_final_exponentiation_hard_part` again from `561254` to `492083` rows
  and `bn254_pairing_check_sample_2_terms` from `1669666` to `1600495`
- `bn254_pairing_check_groth16_style` is the current optimized verifier-shaped
  pairing-core snapshot:
  one variable proof term plus three prepared constant verifier-key terms
- `bn254_pairing_check_sample_2_terms` remains a lower-level primitive sample
  and should not be read as the optimized Groth16 structure

## Suggested Baseline Workflow

Before starting optimization work:

1. capture `profile-layout` output on the main branch
2. save the TSV artifact
3. rerun the same command after each optimization branch
4. diff the rows that correspond to the block you changed

This keeps optimization work grounded in hard measurements instead of intuition.

Recent example worth remembering:

- a `linear_combination(...)` rewrite of `Fp2 * const`, `Fp6` nonresidue
  multiplication, and the `Fp12` `3t +/- 2z` helpers looked locally plausible
  but made the retained baseline worse, including
  `fp12 cyclotomic square` (`1622 -> 1886` rows),
  `final exponentiation` (`587420 -> 678119` rows), and
  `pairing check` (`1682524 -> 1805233` rows)
