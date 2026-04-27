# Cyclotomic Unitary Kernel Design

## Purpose

This document proposes the next structural optimization candidate for the BN254
final-exponentiation hard part after the retained signed-window
`exp_by_neg_x(...)` chain improvement.

The target is the repeated multiply shape:

- `cyclotomic * unitary_inverse(cyclotomic)`

which currently appears multiple times in the hard part.

This is a design document, not an accepted decision record. It exists to make
the implementation boundary, algebraic assumptions, and measurement plan
explicit before code changes land.

Current status:

- the first incremental circuit prototype was attempted only at `y7`
- that prototype was reverted because it regressed measured cost

## Current Situation

The current hard part in
`crates/wrapper-circuits/src/bn254/g2/miller.rs` now benefits from:

- cyclotomic squaring
- precomputed-sum Fp12 multiplication
- precomputed-diff multiplication by unitary inverse
- a retained signed-window `exp_by_neg_x(...)` chain

The current measured split from `profile-layout --family blocks` is:

- `bn254_final_exponentiation_easy_part`: `12288` rows
- `bn254_final_exponentiation_hard_part`: `561254` rows
- `bn254_final_exponentiation`: `574562` rows

Inside the hard part, the following calls still use the general quadratic-over-
`Fp6` kernel specialized only by sums/diffs:

- `y7 = y4 * unitary_inverse(y6)`
- `y8 = y7 * unitary_inverse(y3)`
- `y15 = y9 * unitary_inverse(r)`

Those sites are all currently routed through:

- `AssignedFp12::mul_by_unitary_inverse_with_precomputed_sums(...)`

That helper is already better than a generic `Fp12` multiply, but it still
computes the product in the full `Fp12 = Fp6[w] / (w^2 - v)` model using:

- `a_a = c0 * d0`
- `b_b = c1 * d1`
- `cross = (c0 + c1) * (d0 - d1)`

So the retained path still pays roughly three `Fp6` multiplications per call.

## Algebraic Observation

For cyclotomic-subgroup elements in a quadratic-over-cubic degree-12 tower, one
can compress an element `x = c0 + c1 * w` into a torus coordinate in `Fp6`.

With `w^2 = v`, define:

```text
t(x) = (c0 + 1) / c1
```

and recover:

```text
c0 = (t^2 + v) / (t^2 - v)
c1 = 2t / (t^2 - v)
```

For unitary elements:

- `unitary_inverse(x)` corresponds to `t -> -t`

For the specific multiply shape we care about:

```text
t(x * y^{-1}) = (v - t(x)t(y)) / (t(x) - t(y))
```

This identity was numerically validated during the design pass.

## Important Design Conclusion

This torus identity is promising, but not as a tiny local replacement of one
existing call site.

If we replace a single
`mul_by_unitary_inverse_with_precomputed_sums(...)` call with:

1. compress `x`
2. compress `y`
3. perform the torus operation
4. decompress the result

then we introduce multiple `Fp6` inversions and likely lose the row-count race.

So the promising direction is **not**:

- "drop in a torus kernel for one multiply"

The promising direction **is**:

- keep a short run of hard-part intermediates in torus/compressed form
- only pay compression/decompression at the edges of that run

## Proposed Scope

The best candidate region is the middle hard-part run around:

- `y4`
- `y6`
- `y7`
- `y8`
- `y9`
- `y10`

Reasons:

- it contains repeated `cyclotomic * unitary_inverse(cyclotomic)` products
- it stays inside the cyclotomic subgroup
- it avoids immediate Frobenius boundary crossings on every step
- it is narrow enough to prototype without rewriting the whole hard part

## Proposed Refactor Shape

### Phase 1: host/reference-only prototype

Add host-side helpers for:

- `fp12_cyclotomic_compress_constant(...) -> Fp6Constant`
- `fp12_cyclotomic_decompress_constant(...) -> Fp12Constant`
- `fp12_torus_mul_by_inverse_constant(...) -> Fp6Constant`

Then re-express the candidate hard-part run on the host side and compare:

- output equality against the retained baseline
- operation counts at the formula level

### Phase 2: circuit prototype on one site

Add circuit-side helpers:

- `AssignedFp12::cyclotomic_compress(...) -> AssignedFp6`
- `AssignedFp12::cyclotomic_decompress(...) -> AssignedFp12`
- `AssignedFp6::torus_mul_by_inverse(...) -> AssignedFp6`

Use them on exactly one site first:

- `y7 = y4 * unitary_inverse(y6)`

Measure:

- `final_exponentiation_hard_part`
- `final_exponentiation`

If this isolated version loses, revert early.

Prototype result:

- reverted
- `bn254_final_exponentiation_hard_part`: `561254 -> 571604`
- `bn254_final_exponentiation`: `574562 -> 584912`
- `bn254_pairing_check_sample_2_terms`: `1669666 -> 1680016`
- `bn254_pairing_check_groth16_style`: `1936380 -> 1946730`

Interpretation:

- a one-site torus replacement does not amortize compression/decompression cost
- future work should skip isolated call-site substitutions and only revisit
  this direction if we are willing to keep a longer run of intermediates in
  compressed form

### Phase 3: short-run torus region

If the single-site prototype suggests promise, keep the intermediates compressed
across:

- `y7`
- `y8`
- and possibly the `y9` / `y10` derivation boundary

The goal is to amortize compression/decompression cost.

## Constraints And Risks

### 1. Compression is not free

The torus map uses division in `Fp6`, so any win depends on amortizing that
cost across multiple downstream operations.

### 2. Decompression is also not free

The inverse map needs:

- `t^2`
- `t^2 - v`
- inversion in `Fp6`
- several `Fp6` / `Fp12`-level products

So an isolated one-off kernel is unlikely to win.

### 3. Frobenius boundaries matter

The hard part later applies:

- `frobenius_map(1)`
- `frobenius_map(2)`
- `frobenius_map(3)`

If torus form interacts poorly with those transitions, the compressed region
should stop before them.

### 4. Measurement must dominate elegance

This design should only ship if it lowers real rows in:

- `bn254_final_exponentiation_hard_part`
- and ideally `bn254_final_exponentiation`

No formula should land merely because it is algebraically attractive.

## Acceptance Criteria

The design should be considered successful only if the retained implementation:

1. preserves host/reference equality with the current hard-part output
2. preserves circuit correctness on the existing tests
3. reduces `bn254_final_exponentiation_hard_part` rows versus the retained
   signed-window baseline of `561254`
4. does not regress `pairing check` totals enough to erase the hard-part win

## Decision Gate

Proceed to implementation only if we agree on this narrower framing:

- **not** “invent a one-off torus multiply kernel”
- **yes** “prototype a short compressed torus region for the repeated
  `cyclotomic * unitary_inverse(cyclotomic)` run inside the hard part”

That is the proposal this document is asking to approve.

After the first prototype result, the decision gate is narrower still:

- do **not** retry the `y7`-only substitution
- only proceed if we want to prototype a multi-step compressed region
