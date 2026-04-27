# ADR 0002: BN254 Local Optimization Policy For The Pairing Core

## Status

Accepted.

## Context

The BN254 pairing-core lane is now implemented far enough that optimization work
must be driven by measured circuit costs rather than algebraic intuition alone.
Several local Midnight primitives looked promising on paper, but they did not
all help once expressed through the current foreign-field stack.

The repository also now depends on a fixed `exp_by_neg_x(...)` chain inside the
BN254 final-exponentiation hard part. That chain is shared across host and
circuit code and materially affects pairing-check cost.

We need one durable decision record that captures:

- which local optimization directions are retained
- which ones were tried and explicitly ruled out
- how future optimization work should be evaluated

## Decision

### 1. Keep `mul_by_constant(...)` as the default local optimization primitive

Retained rule:

- prefer `FieldChip::mul_by_constant(...)` for small fixed multipliers in the
  BN254 tower and pairing path

Reason:

- it has repeatedly reduced rows in real measured circuits
- it remains the best-performing simple local primitive discovered so far

### 2. Retain the `add_constant(...)` rewrite on the G2 on-curve path

Retained rewrite:

- use `FieldChip::add_constant(...)` for the fixed BN254 twist coefficient in
  `AssignedG2Affine::assert_on_curve(...)`

Measured effect:

- `g2 on_curve`: `400 -> 378`
- `g2 neg`: `930 -> 886`
- `g2 proj from_affine`: `970 -> 948`
- `g2 proj double`: `2594 -> 2550`
- `g2 proj add`: `4582 -> 4516`
- `g2 double_with_line`: `2698 -> 2654`
- `g2 mixed_add_with_line`: `3374 -> 3330`

Interpretation:

- this is a retained local G2 / Miller-prep win
- it does not by itself move the pairing-core block totals

### 3. Reject the tested `linear_combination(...)` rewrites on the tower hot path

Rejected rewrite family:

- rewriting `AssignedFp2::mul_by_constant(...)`
- rewriting `AssignedFp6::mul_by_nonresidue_fp2(...)`
- rewriting the Fp12 `3t +/- 2z` helpers

Measured regressions relative to the retained baseline included:

- `fp12 cyclotomic square`: `1622 -> 1886`
- `final exponentiation`: `587420 -> 678119`
- `pairing check`: `1682524 -> 1805233`

Interpretation:

- the obvious foreign-field `linear_combination(...)` rewrites are not retained
- do not re-land this family without new measurement evidence and a materially
  different constraint shape

### 4. Reject the tested `select` / `is_equal*` / `is_zero` cleanup for GT identity

Rejected optimization framing:

- replacing the manual final GT identity check with composite `Fp2` / `Fp6` /
  `Fp12` boolean equality helpers

Measured effect:

- row-neutral in `wrapper-cli doctor`
- row-neutral in `profile-layout --family blocks`

Interpretation:

- this family may still help readability or future branching logic
- it is not retained as a performance optimization

### 5. Retain the signed-window `exp_by_neg_x(...)` chain

Retained chain:

- start at `35`
- `<< 6, -35`
- `<< 9, +101`
- `<< 8, -83`
- `<< 9, +37`
- `<< 9, +105`
- `<< 11, +79`
- `<< 5, +17`

Why this chain was retained:

- it spends one extra cyclotomic square in precomputation
- it removes one main-chain multiplication per `exp_by_neg_x(...)` call
- `exp_by_neg_x(...)` is called three times in the hard part
- at the current cost model, that trade wins

Measured effect:

- `final exponentiation hard part`: `574112 -> 561254`
- `final exponentiation`: `587420 -> 574562`
- `pairing check` sample: `1682524 -> 1669666`
- `pairing check` Groth16-style: `1949238 -> 1936380`

Implementation rule:

- keep `crates/wrapper-circuits/src/bn254/final_exp_chain.rs` as the canonical
  source of truth for the chain metadata
- host and circuit paths must continue consuming the same chain description

### 6. Evaluate future local optimizations by measured circuit cost, not elegance

Required evaluation path:

1. run `cargo run -q -p wrapper-cli -- doctor`
2. run `cargo run -q -p wrapper-cli -- profile-layout --family blocks`
3. compare against the retained baseline before keeping the rewrite

## Consequences

Positive:

- future optimization work starts from measured facts instead of repeated
  rediscovery
- the repository now has a durable record of both retained wins and rejected
  hypotheses
- the signed-window `exp_by_neg_x(...)` chain is clearly established as the
  current best known local hard-part schedule

Tradeoffs:

- the retained chain is more complex than the earlier all-positive schedule
- some tempting local rewrites are now explicitly off-limits unless backed by
  fresh data
- documentation must stay synchronized with new measured baselines

## Alternatives Considered

Keep only operational notes in `AGENTS.md` and optimization docs:

- rejected because these optimization results affect cryptographic
  implementation policy, not just per-session operator guidance

Prefer algebraically simpler but slower chains or helper rewrites:

- rejected because the current repository optimizes for measured circuit cost,
  not minimal conceptual novelty
