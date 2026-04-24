# Midnight Local Optimization Notes

This note records which `midnight-circuits` / `midnight-proofs` primitives have
already proven useful for reducing rows in the current BN254 tower and Groth16
verifier slice, and which ones look most promising for future local
optimizations.

Scope:

- local or semi-local optimizations only
- current Halo2 / Midnight BN254 primitive path only
- focus on row count first, profiling time second

## Current Status

The highest-value local optimization discovered so far was:

- replace generic foreign-field multiplies by small fixed constants with
  `FieldChip::mul_by_constant(...)`

This was especially effective in:

- `mul_by_nonresidue_fp2`

That change materially reduced the verifier and was enough to move the current
Groth16 verifier total from `k = 22` down to `k = 21`.

## Confirmed Useful Midnight Primitives

## 1. `mul_by_constant`

Priority: highest

Why it matters:

- it is already proven to reduce rows materially in the current BN254 tower
- it is the best local primitive discovered so far
- Midnight's foreign-field implementation has a dedicated fast path for small
  constants instead of always falling back to generic multiplication

Best use cases:

- tower helpers with small fixed constants
- repeated transforms such as `* 9`
- fixed `Fp2` / `Fp` coefficient multiplies in the pairing path

## 2. `linear_combination`

Priority: high

Why it matters:

- Midnight documents this as potentially more efficient than manually chaining
  `mul_by_constant` plus `add`
- many tower transforms are just affine linear maps over already-assigned limbs

Best use cases:

- `Fp2` helpers whose output coordinates are linear combinations of the input
  coordinates with small constants
- replacements for short “multiply by constant, then add/sub” chains

Likely follow-up target:

- a tighter version of `AssignedFp2::mul_by_constant(...)`

## 3. `add_constant` / `add_constants`

Priority: high

Why it matters:

- pairs naturally with `mul_by_constant`
- useful when a helper is mostly “small linear transform plus offset”

Best use cases:

- affine transforms in the tower
- repeated coordinate adjustments with fixed offsets

## 4. `select`, `is_equal`, `is_equal_to_fixed`, `is_zero`

Priority: medium

Why it matters:

- useful for local cleanup of branchy helpers and special cases
- can reduce some wrapper overhead when comparisons are frequent

Best use cases:

- inversion helpers
- case splits on fixed values
- keeping unsupported cases explicit without broadening public APIs

## 5. decomposition / canonicity / biguint gadgets

Priority: medium-low

Why it matters:

- powerful infrastructure
- not the current main bottleneck

Best use cases:

- future gadgets that need stronger bound / limb / canonicity handling
- later phases, not current pairing-core hotspot work

## Things That Look Less Promising Right Now

## 1. `fp6_frobenius_map` / `fp12_frobenius_map` direct rewrites

Current reason:

- in the current circuit path, `AssignedFp12::frobenius_map(...)` is
  witness-driven and mostly assignment-based rather than an expensive explicit
  algebraic transform inside the circuit
- “optimizing” that helper locally is therefore unlikely to produce the same
  kind of win as `mul_by_nonresidue_fp2`

Takeaway:

- optimize Frobenius-related arithmetic only where the multiplication by fixed
  coefficients actually happens in-circuit
- do not assume the public `frobenius_map(...)` helpers are the right hotspot

## 2. schedule-only Miller accumulator refactors

Current reason:

- a local attempt to fuse accumulator square and line consumption did not reduce
  rows materially

Takeaway:

- future Miller-loop optimization should target underlying arithmetic cost, not
  just sequencing wrappers

## 3. generic “use lookups” as a strategy

Current reason:

- the Midnight stack clearly supports lookup-heavy gadgets in other domains
  (for example SHA-like chips)
- but there is no obvious ready-made lookup gadget in this repo for foreign
  field tower arithmetic

Takeaway:

- lookups are viable as an implementation technique
- but the optimization target should still be a specific repeated tower
  operation, not “lookups” in the abstract

## Prioritized Next Local Targets

## 1. `AssignedFp2::mul_by_constant(...)`

Why this is next:

- it is the natural companion to the successful `mul_by_nonresidue_fp2` rewrite
- it can probably be reduced further with `linear_combination(...)`
- it feeds directly into fixed `Fp2` coefficient multiplies in the pairing path

## 2. more fixed-constant `Fp2` multiplies in the variable Miller path

Why this is next:

- the variable `proof.b` path still pays real cost
- `g2_mul_by_char(...)` already benefited from replacing generic `Fp2 * const`
  multiplications with constant-aware helpers

## 3. other small fixed transforms in the tower

Why this is next:

- once a reusable pattern exists for one helper, the same Midnight primitives
  can often be reused elsewhere

## Practical Guidance

When evaluating a local optimization idea, ask these questions first:

1. Is the expensive work actually happening in-circuit, or is the helper mostly
   witness assignment?
2. Does the operation multiply by a small or fixed constant?
3. Can the output be expressed as a short linear combination of already-assigned
   values?
4. Is the operation repeated enough times that a local improvement will
   compound across the verifier?

If the answer is “yes” to questions 2 through 4, it is probably a good local
Midnight optimization candidate.
