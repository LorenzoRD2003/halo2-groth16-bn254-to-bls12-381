# Pairing Kernel Opportunity Audit Plan

## Purpose

This document turns the current pairing-kernel audit into an implementation
plan for the next class of BN254 pairing-core experiments.

The main question is narrow and strict:

- can the repository still land a new arithmetic kernel that reduces real
  foreign-field multiplication count in the current hot pairing path?

This plan is intentionally not about:

- selector cleanup
- chip repackaging
- witness reshaping by itself
- reducing only the number of helper calls while keeping the same base
  arithmetic count

For this plan, a candidate is only considered promising if it has a credible
path to lowering real `Fp2` / `Fp6` multiplication work, not just reorganizing
how the current work is expressed.

## Current Situation

The current retained baseline already includes:

- compressed cyclotomic squaring inside `exp_by_neg_x(...)`
- prepared constant G2 Miller lines for fixed verifier-key G2 terms
- full precomputation of the fixed verifier-key term `e(alpha, beta)` into a
  GT constant for the Groth16-style pairing block

Current relevant measured baselines:

- `bn254_final_exponentiation_hard_part`: `492083`
- `bn254_final_exponentiation`: `505391`
- `bn254_pairing_check_sample_2_terms`: `1600045`
- `bn254_pairing_check_groth16_style`: `1632579`

These values are the current “beat this or do not land it” reference points for
kernel work.

## Current Hot Kernels

### 1. Generic quadratic-over-`Fp6` multiply family

The current `Fp12` multiply helpers still use the same core three-product shape:

- `a_a = c0 * d0`
- `b_b = c1 * d1`
- `cross = (c0 +/- c1) * (d0 +/- d1)`

Current code paths:

- `AssignedFp12::mul_with_precomputed_sums(...)`
- `AssignedFp12::mul_by_unitary_inverse_with_precomputed_sums(...)`
- `AssignedFp12::frobenius_mul_with_precomputed_rhs_sum(...)`

The first two are already useful relative to a fully generic path, but they
still pay roughly three `Fp6` multiplies each.

### 2. Sparse line accumulation

The current line-accumulation path already uses a sparse `mul_by_034`-style
shape:

- `AssignedMillerAccumulator::mul_by_line(...)`
- `AssignedFp6::mul_by_01(...)`

This is already a real specialization, not a placeholder generic `Fp12` mul.

### 3. Cyclotomic square

The current cyclotomic square is already specialized and already won:

- `AssignedFp12::cyclotomic_square(...)`
- `AssignedFp12::compressed_cyclotomic_square_n_times(...)`

So the remaining question is mostly about multiply kernels, not square kernels.

## Audit Conclusions

### Highest-confidence opportunity: `frobenius(cyclotomic) * cyclotomic`

Why it is still live:

- the current helper still materializes the Frobenius image as witness data
  before multiplying
- Frobenius in this tower is mostly conjugation, sign flips, slot movement, and
  multiplication by fixed constants
- the hard part hits this pattern three times in a row-sensitive region

Why this is promising under the strong criterion:

- if the Frobenius action is pushed into a dedicated multiply kernel, some of
  the current `Fp6` work may disappear instead of merely moving around
- this is more promising than a cosmetic `frobenius_map(...)` refactor because
  the multiply shape itself can change

Primary sites:

- `y12 = frobenius(y9, 1) * y11`
- `y14 = frobenius(y8, 2) * y12`
- `y15 = frobenius(y9 * unitary_inverse(r), 3) * y14`

### Second-best opportunity: `cyclotomic * unitary_inverse(cyclotomic)`

Why it is still live:

- the unitary inverse itself is already cheap, because it is just conjugation
- the cost is in the multiply that follows, and the current helper still uses
  the generic three-`Fp6`-product family

Why this is weaker than the Frobenius candidate:

- the repository already tried torus-style work and lost when the attempt did
  not amortize compression/decompression over a long enough region
- any new attempt here must prove an arithmetic win without relying on local
  compression/decompression overhead being “probably fine”

This area is still worth revisiting, but only with a clearly different kernel,
not a small re-expression of the old one.

### Third-best opportunity: `cyclotomic * cyclotomic`

Why it is still still open:

- the current helper is still the generic three-product family with subgroup-
  aware sums
- these products appear repeatedly in the hard part

Why it ranks below the first two:

- a previous `CyclotomicFp12MulChip` rollout was nearly neutral, which strongly
  suggests that “same arithmetic, new wrapper” is exhausted
- any future win here probably needs a genuinely different formula, not a
  different packaging

### Lower-confidence opportunity: `mul_by_034` / sparse line accumulation

Why it ranks lower:

- the current path is already substantially specialized
- `AssignedFp6::mul_by_01(...)` is already sparse and algebra-aware
- the room for improvement appears smaller unless a new formula can eliminate
  real `Fp2` multiplies

This area should not be ignored forever, but it should not be the next default
experiment.

## Things That Should Not Count As Progress

The following do not satisfy the bar for this plan unless accompanied by a real
 arithmetic reduction:

- replacing helpers with a new chip name
- moving sums and diffs into a different method boundary
- reducing witness materialization while keeping the same multiply kernel
- fusing gates if the same number of `Fp2` / `Fp6` multiplies still occur
- switching to a compressed or torus representation for one isolated site

Those changes may still be useful later, but they do not answer the current
question.

## Primary Targets By Priority

### Priority 1. Fused Frobenius-multiply kernel

Goal:

- design a dedicated kernel for `frobenius(x, power) * y` where `x` stays in
  the cyclotomic subgroup and `power in {1, 2, 3}`

Success condition:

- lower real multiplication count relative to the current
  `frobenius_mul_with_precomputed_rhs_sum(...)` helper

Primary files:

- `crates/wrapper-circuits/src/bn254/fp12.rs`
- `crates/wrapper-circuits/src/bn254/host/mod.rs`
- `crates/wrapper-circuits/src/bn254/g2/miller.rs`

### Priority 2. New kernel for `cyclotomic * unitary_inverse(cyclotomic)`

Goal:

- beat `mul_by_unitary_inverse_with_precomputed_sums(...)` with an arithmetic
  kernel that reduces real multiply count

Success condition:

- no isolated compress/decompress substitution
- clear formula-level evidence that the number of `Fp6` or `Fp2` multiplies is
  lower than the current path

Primary files:

- `crates/wrapper-circuits/src/bn254/fp12.rs`
- `crates/wrapper-circuits/src/bn254/host/mod.rs`
- `docs/plans/0002-cyclotomic-unitary-kernel-design.md`

### Priority 3. New kernel for `cyclotomic * cyclotomic`

Goal:

- determine whether the current subgroup multiply family can be replaced by a
  strictly cheaper arithmetic formula rather than a repackaging

Success condition:

- the formula-level count must beat the current three-`Fp6`-product family

Primary files:

- `crates/wrapper-circuits/src/bn254/fp12.rs`
- `crates/wrapper-circuits/src/bn254/host/mod.rs`

### Priority 4. Re-audit `mul_by_034` only after the hard-part kernels

Goal:

- verify whether any remaining sparse-line algebra can lower true `Fp2`
  multiplication count beyond the current `mul_by_01(...)` + `mul_by_034` path

Success condition:

- explicit reduction in the sparse-path arithmetic count, not just improved
  accumulator structure

Primary files:

- `crates/wrapper-circuits/src/bn254/fp6.rs`
- `crates/wrapper-circuits/src/bn254/g2/miller.rs`

## Implementation Phases

## Phase 1. Formula-level host audit

Goal:

- write down exact arithmetic counts for the current kernels and each proposed
  candidate before changing the circuit code

Deliverables:

- current count table for:
  `frobenius_mul_with_precomputed_rhs_sum(...)`,
  `mul_by_unitary_inverse_with_precomputed_sums(...)`,
  `mul_with_precomputed_sums(...)`,
  `mul_by_01(...)`
- candidate count table for each replacement formula
- a yes/no recommendation on whether the candidate can possibly win before
  Halo2 overhead is considered

Acceptance criteria:

- at least one candidate shows a believable base-arithmetic win on paper
- otherwise stop the line early and do not proceed to Phase 2

## Phase 2. Host/reference prototype

Goal:

- implement the most promising candidate only on the host/reference side

Scope:

- add a constant/reference helper under `bn254/host/`
- compare output equality against the retained host baseline on randomized
  samples

Acceptance criteria:

- exact equality on randomized samples
- formula-level count still favorable after concrete derivation is complete

Stop condition:

- if the concrete formula ends up matching current multiply count, stop

## Phase 3. Isolated circuit prototype

Goal:

- land exactly one circuit-side experimental kernel in the narrowest useful site

Recommended first site:

- one of the three hard-part `frobenius(...) * cyclotomic` calls

Why:

- this is the highest-confidence remaining opportunity
- it avoids reopening torus-region complexity too early

Acceptance criteria:

- correctness tests pass
- no regressions in the primitive sanity circuits
- at least one block-level row win in:
  `bn254_final_exponentiation_hard_part`,
  `bn254_final_exponentiation`,
  `bn254_pairing_check_groth16_style`

Stop condition:

- if the isolated circuit prototype is flat or worse, revert and move to the
  next priority family instead of broadening the rollout

## Phase 4. Short-run rollout

Goal:

- if Phase 3 wins, apply the same kernel family to the rest of the matching
  sites in the hard part

Expected scope:

- all three `frobenius(...) * cyclotomic` sites, or
- all repeated instances of the specific multiply family that won

Acceptance criteria:

- the full hard part still improves relative to the pre-rollout baseline
- `bn254_pairing_check_groth16_style` also improves or stays proportional to
  the block win

## Metrics To Track

Primary metrics:

- `bn254_final_exponentiation_hard_part`
- `bn254_final_exponentiation`
- `bn254_pairing_check_groth16_style`

Secondary metrics:

- `bn254_pairing_check_sample_2_terms`
- `bn254_miller_loop_narrow`
- `groth16_fixture_verifier_total`

Measurement commands:

```bash
cargo run -p wrapper-cli -- profile-layout --family blocks
cargo run -p wrapper-cli -- profile-layout --family groth16
```

## Decision Rules

Land the work only if both are true:

- the candidate reduces formula-level multiply count in a way that survives
  concrete implementation
- the measured block rows go down on the retained baseline

Do not land the work if either is true:

- the improvement is only packaging or materialization cleanup
- the circuit result is neutral or regresses against the current retained rows

## Recommended First Experiment

The first experiment after this plan should be:

- design and count a dedicated `frobenius(cyclotomic) * cyclotomic` kernel for
  powers `1`, `2`, and `3`

Rationale:

- it hits repeated hard-part sites
- it has the best structural chance to remove real work
- it has not yet been disproven by prior measurements the way isolated torus
  substitution and subgroup-multiply repackaging already were
