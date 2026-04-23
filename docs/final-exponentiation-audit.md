# Final Exponentiation Audit

This note audits the current BN254 final exponentiation implementation in the
pairing core as it exists in the repository today.

Scope:

- describe the exact code-level operation chain
- decompose it into easy part and hard part
- record real layout measurements for those sub-blocks
- identify grounded optimization candidates for a later focused rewrite

This is an audit document, not an optimization PR.

## Source Of Truth

Primary implementation:

- `crates/wrapper-circuits/src/bn254/g2/miller.rs`

Host/reference mirror:

- `crates/wrapper-circuits/src/bn254/host/pairing_host.rs`

Low-level Fp12 operations used by the chain:

- `crates/wrapper-circuits/src/bn254/fp12.rs`

## Current Code-Level Structure

The current implementation is now split explicitly into:

- `final_exponentiation_easy_part(...)`
- `final_exponentiation_hard_part(...)`
- `final_exponentiation(...) = hard_part(easy_part(value))`

This split preserves the original semantics and order of operations.

## Exact Operation Chain

## Easy Part

Input: `value`

Code-level sequence:

1. `f1 = value.unitary_inverse(...)`
2. `f2 = value.inv(...)`
3. `r = f1.mul(..., &f2)`
4. `r_clone = r.clone()`
5. `r = r.frobenius_map(..., 2)`
6. `r = r.mul(..., &r_clone)`

Code-level operation tally:

- `unitary_inverse`: 1
- `inv`: 1
- `mul`: 2
- `frobenius_map`: 1
- `square`: 0

Notes:

- in this codebase, `unitary_inverse(...)` is implemented as `conjugate(...)`
  on `Fp12`
- `inv(...)` is not a cheap primitive; in `fp12.rs` it assigns an inverse
  witness, multiplies it back by the original value, and constrains the product
  to equal one

## Hard Part

Input: `r = easy_part(value)`

Code-level sequence:

1. `y0 = exp_by_neg_x(r)`
2. `y1 = y0.square()`
3. `y2 = y1.square()`
4. `y3 = y2.mul(y1)`
5. `y4 = exp_by_neg_x(y3)`
6. `y5 = y4.square()`
7. `y6 = exp_by_neg_x(y5)`
8. `y3 = y3.unitary_inverse()`
9. `y6 = y6.unitary_inverse()`
10. `y7 = y6.mul(y4)`
11. `y8 = y7.mul(y3)`
12. `y9 = y8.mul(y1)`
13. `y10 = y8.mul(y4)`
14. `y11 = y10.mul(r)`
15. `y12 = y9.frobenius_map(1)`
16. `y12 = y12.mul(y11)`
17. `y8 = y8.frobenius_map(2)`
18. `y14 = y8.mul(y12)`
19. `r_inv = r.unitary_inverse()`
20. `y15 = r_inv.mul(y9)`
21. `y15 = y15.frobenius_map(3)`
22. `out = y15.mul(y14)`

Code-level operation tally outside `exp_by_neg_x(...)`:

- `square`: 3
- `mul`: 10
- `frobenius_map`: 3
- `unitary_inverse`: 3

## What `exp_by_neg_x(...)` Does

`exp_by_neg_x(value)` is implemented as:

1. `pow_constant_exp(value, &[4965661367192848881])`
2. `unitary_inverse(...)`

The current `pow_constant_exp(...)` is a generic square-and-multiply loop in
`fp12.rs`, not a BN254-specific handcrafted chain.

For the current hard-coded exponent:

- exponent bit length: `63`
- exponent popcount: `28`

So each `pow_constant_exp(...)` performs:

- `62` squares
- `27` multiplies

Then `exp_by_neg_x(...)` adds:

- `1` `unitary_inverse`

Since the hard part calls `exp_by_neg_x(...)` three times, that contributes:

- `186` squares
- `81` multiplies
- `3` `unitary_inverse`

## Total Hard-Part Operation Tally

Adding the explicit hard-part operations to the three `exp_by_neg_x(...)`
calls gives:

- `square`: `189`
- `mul`: `91`
- `frobenius_map`: `3`
- `unitary_inverse`: `6`
- `inv`: `0`

This explains why the hard part dominates the total block cost.

## Real Layout Measurements

Measured with:

```bash
cargo run -p wrapper-cli -- profile-layout --family blocks
```

Current rows:

- `bn254_final_exponentiation_easy_part`: `13884` rows, `k=14`
- `bn254_final_exponentiation_hard_part`: `1190996` rows, `k=21`
- `bn254_final_exponentiation`: `1215080` rows, `k=21`

Interpretation:

- the easy part is tiny relative to the total
- the hard part accounts for essentially all of the current final
  exponentiation cost
- future optimization work should focus almost entirely on the hard part unless
  a very cheap easy-part cleanup appears

## Reuse / Recomputation Findings

Grounded observations from the current code:

### 1. `exp_by_neg_x(...)` is repeated three times

This is the largest structural repetition in the current code.

The same generic square-and-multiply engine is invoked on:

- `r`
- `y3`
- `y5`

Each call is expensive on its own.

### 2. No obvious duplicated Frobenius image of the same expression

The current Frobenius calls are:

- `r.frobenius_map(2)` in the easy part
- `y9.frobenius_map(1)`
- `y8.frobenius_map(2)`
- `y15.frobenius_map(3)`

These are on different values, so there is no obvious trivial reuse to extract
without changing the chain.

### 3. No obvious duplicated inverse of the same expression

The current inverse-like steps are:

- `value.inv(...)` in the easy part
- `y3.unitary_inverse(...)`
- `y6.unitary_inverse(...)`
- `r.unitary_inverse(...)`

These all apply to different values.

### 4. Some values are reused already

The current code already reuses:

- `r` across many hard-part multiplies
- `y1` in two later multiplies
- `y4` in two later multiplies
- `y8` before and after a Frobenius image
- `y9` both directly and through a Frobenius image

So the biggest remaining opportunity is not trivial local memoization; it is
changing the structure of the exponentiation path itself.

## Key Cost Observation

The current final exponentiation is not expensive because of the easy part.

It is expensive because the hard part contains:

- three generic constant-exponentiation calls
- a very large number of Fp12 squares and multiplies
- enough structure to push the block to `k=21`

## Ranked Next Optimization Candidates

These are candidates for a later targeted rewrite, not changes implemented in
this audit.

### 1. Replace generic `exp_by_neg_x(...)` with a handcrafted BN254 chain

Why it ranks first:

- it is called three times
- each call currently expands to `62` squares + `27` multiplies + one
  `unitary_inverse`
- it is the clearest repeated structural hotspot in the code

### 2. Introduce cyclotomic-squaring-aware hard-part rewriting

Why it ranks high:

- the hard part is square-heavy
- if the relevant values are in the expected subgroup domain, specialized
  squaring should have high leverage

Caveat:

- this should be validated carefully against the exact values the current chain
  produces; do not assume every intermediate can be replaced blindly

### 3. Evaluate compressed squaring within the hard part

Why it ranks high:

- same reason as above: the hard part is dominated by repeated squaring
- this is the other obvious square-focused structural lever after cyclotomic
  squaring

### 4. Rebuild the hard part around a better addition chain

Why it matters:

- even without new algebraic kernels, the current chain may not be the best
  circuit-oriented arrangement of exponentiations and intermediate products

### 5. Treat easy-part cleanup as low priority

Why it ranks low:

- easy part is only `13884` rows versus `1190996` for the hard part
- unless a cleanup is nearly free, it is unlikely to move total verifier cost
  materially

## Commands To Re-Run The Audit

Compile:

```bash
cargo check
```

Validate the new decomposition circuits:

```bash
cargo test -p wrapper-circuits final_exponentiation_easy_part_sample_matches_host_decomposition -- --ignored --nocapture
cargo test -p wrapper-circuits final_exponentiation_hard_part_sample_matches_host_decomposition -- --ignored --nocapture
```

Measure the current block:

```bash
cargo run -p wrapper-cli -- profile-layout --family blocks
```

## Caveats

- this audit is layout-focused, not runtime-focused
- `pow_constant_exp(...)` is counted structurally from the current code path,
  not from a symbolic algebra system
- this note does not claim the current chain is mathematically suboptimal in an
  abstract sense; it only identifies where the implemented circuit is spending
  its cost
