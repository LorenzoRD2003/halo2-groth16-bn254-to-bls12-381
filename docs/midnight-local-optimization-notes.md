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

The most recent retained pairing-core win was:

- replace repeated full cyclotomic squaring blocks inside `exp_by_neg_x(...)`
  with compressed cyclotomic squaring plus verified decompression

That rewrite improved the hard part and every final pairing-facing total that
depends on it:

- `final exponentiation hard part`: `561254 -> 492083`
- `final exponentiation`: `574562 -> 505391`
- `pairing check` sample: `1669666 -> 1600495`
- `pairing check` Groth16-style: `1936380 -> 1867209`

Takeaway:

- the first subgroup-aware direction that materially beat the retained signed
  chain and generic cyclotomic arithmetic was not a new multiply kernel, but
  compressed squaring amortized over the long square blocks in `exp_by_neg_x(...)`
- this is now the strongest retained local optimization in the pairing-core
  lane because it lowers both rows and `k` for final exponentiation

The previous retained pairing-core win was:

- replace the old positive-window `exp_by_neg_x(...)` chain with a signed-window
  chain that spends one extra cyclotomic square in precomputation to save one
  main-chain multiplication per call

The retained signed chain starts from `35` and then consumes:

- `<< 6, -35`
- `<< 9, +101`
- `<< 8, -83`
- `<< 9, +37`
- `<< 9, +105`
- `<< 11, +79`
- `<< 5, +17`

That rewrite improved the hard part and every final pairing-facing total that
depends on it:

- `final exponentiation hard part`: `574112 -> 561254`
- `final exponentiation`: `587420 -> 574562`
- `pairing check` sample: `1682524 -> 1669666`
- `pairing check` Groth16-style: `1949238 -> 1936380`

Takeaway:

- at the current cost model, one fewer cyclotomic-subgroup multiplication per
  `exp_by_neg_x(...)` call is worth more than one extra cyclotomic square in
  precomputation
- `exp_by_neg_x(...)` is still the dominant local hotspot inside the hard part,
  but its fixed chain is now materially better than the earlier all-positive
  window schedule

The most recent measured local win was:

- use `FieldChip::add_constant(...)` for the fixed BN254 G2 twist coefficient
  in `AssignedG2Affine::assert_on_curve(...)` instead of assigning `b` as an
  `Fp2` witness and then adding it as a variable term

That change improved the current curve / Miller-prep slice while leaving the
pairing-core totals unchanged:

- `g2 on_curve`: `400 -> 378`
- `g2 neg`: `930 -> 886`
- `g2 proj from_affine`: `970 -> 948`
- `g2 proj double`: `2594 -> 2550`
- `g2 proj add`: `4582 -> 4516`
- `g2 double_with_line`: `2698 -> 2654`
- `g2 mixed_add_with_line`: `3374 -> 3330`

Takeaway:

- `add_constant(...)` is worthwhile when the offset is truly fixed and already
  part of the algebraic definition, like the BN254 twist coefficient
- this is currently a local G2 / prep win, not a pairing-core win:
  `miller loop`, `final exponentiation`, and `pairing check` rows stayed the
  same

The most recent measured non-win was:

- a focused `FieldChip::linear_combination(...)` pass over `AssignedFp2::mul_by_constant(...)`,
  `AssignedFp6::mul_by_nonresidue_fp2(...)`, and the `3t +/- 2z` helpers used
  by Fp12 cyclotomic square

That pass was reverted after profiling because it increased rows across the
current tower / pairing hot path. Relative to the current retained baseline, it
regressed:

- `fp6 mul`: `1252 -> 1318`
- `fp6 square`: `736 -> 802`
- `fp12 mul`: `4076 -> 4307`
- `fp12 square`: `2594 -> 2825`
- `fp12 cyclotomic square`: `1622 -> 1886`
- `miller accumulator square`: `2714 -> 2945`
- `miller accumulator mul_by_line`: `4248 -> 4479`
- `miller accumulator mul_by_line sparse`: `2592 -> 2691`
- `miller loop narrow`: `457060 -> 480457`
- `final exponentiation`: `587420 -> 678119`
- `pairing check`: `1682524 -> 1805233`

Takeaway:

- in Midnight's current foreign-field implementation, `linear_combination(...)`
  is not automatically a win just because the algebra looks affine
- for the specific short BN254 tower transforms above, the retained
  `mul_by_constant(...)`-first rewrites are better than the attempted
  `linear_combination(...)` replacements

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

Current repo-specific caution:

- do not assume `linear_combination(...)` beats hand-written short
  `mul_by_constant(...)` plus `add/sub` chains on the foreign-field path
- the April 27, 2026 pass that rewrote `AssignedFp2::mul_by_constant(...)`,
  `AssignedFp6::mul_by_nonresidue_fp2(...)`, and the `Fp12` `3t +/- 2z`
  helpers was measured and reverted because it made every relevant block worse

## 2a. signed `exp_by_neg_x(...)` windows

Priority: high

Why it matters:

- the hard part still dominates final exponentiation cost
- `exp_by_neg_x(...)` is called three times inside the hard part
- cyclotomic squares are cheaper than cyclotomic-subgroup multiplies in the
  current repo, so a chain that trades one extra square for one fewer multiply
  can win materially
- compressed cyclotomic squaring inside the retained signed chain now wins even
  more materially by shrinking the repeated square blocks themselves

Best use cases:

- fixed exponent chains in the cyclotomic subgroup
- situations where negative windows can be consumed through unitary inverse /
  conjugation instead of a full generic multiply

Status update:

- retained and measured
- compressed cyclotomic squaring is now the preferred implementation for the
  repeated `square_count > 1` blocks inside `exp_by_neg_x(...)`

## 3. `add_constant` / `add_constants`

Priority: high

Why it matters:

- pairs naturally with `mul_by_constant`
- useful when a helper is mostly “small linear transform plus offset”

Best use cases:

- affine transforms in the tower
- repeated coordinate adjustments with fixed offsets

Current repo-specific note:

- the retained win so far is narrow: adding the fixed BN254 G2 twist
  coefficient directly in `AssignedG2Affine::assert_on_curve(...)`
- there is not yet evidence that `add_constant(...)` materially changes the
  final-exponentiation or pairing-check hotspots

## 4. `select`, `is_equal`, `is_equal_to_fixed`, `is_zero`

Priority: medium

Why it matters:

- useful for local cleanup of branchy helpers and special cases
- can reduce some wrapper overhead when comparisons are frequent

Best use cases:

- inversion helpers
- case splits on fixed values
- keeping unsupported cases explicit without broadening public APIs

Current repo-specific note:

- an April 27, 2026 pass that encapsulated the final GT identity check into
  composite `Fp2` / `Fp6` / `Fp12` boolean equality helpers was measured and
  then reverted because it left `wrapper-cli doctor` and
  `profile-layout --family blocks` unchanged in rows
- treat this family as useful for clarity or future branching logic, but not as
  a retained row-count optimization on the current pairing-check path
- a later April 27, 2026 torus-style prototype that replaced only the `y7`
  hard-part site (`cyclotomic * unitary_inverse(cyclotomic)`) also lost after
  measurement, because the compression/decompression overhead outweighed the
  local kernel specialization
- a later broad `CyclotomicFp12MulChip` rollout over the repeated
  `cyclotomic * cyclotomic` hard-part sites (`y3`, `y9`, `y10`, `y11`) also
  lost slightly, so a mere chip-level repackaging of the current ambient Fp12
  multiplication formula is not enough to win rows
- by contrast, compressed cyclotomic squaring *did* win once it was restricted
  to the repeated square blocks in `exp_by_neg_x(...)`, which is the current
  retained subgroup-aware direction

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

Status update:

- attempted and measured
- the straightforward `linear_combination(...)` rewrite regressed rows and was
  reverted
- any future revisit should start from the retained `mul_by_constant(...)`
  version and must show a real `wrapper-cli doctor` win before landing

## 2. more fixed-constant `Fp2` multiplies in the variable Miller path

Why this is next:

- the variable `proof.b` path still pays real cost
- `g2_mul_by_char(...)` already benefited from replacing generic `Fp2 * const`
  multiplications with constant-aware helpers

## 3. other small fixed transforms in the tower

Why this is next:

- once a reusable pattern exists for one helper, the same Midnight primitives
  can often be reused elsewhere

Status update:

- one real fixed-offset win has landed on the G2 on-curve path
- future `add_constant(...)` work should look for similarly honest fixed-offset
  additions, not variable affine combinations disguised as constants

## Practical Guidance

When evaluating a local optimization idea, ask these questions first:

1. Is the expensive work actually happening in-circuit, or is the helper mostly
   witness assignment?
2. Does the operation multiply by a small or fixed constant?
3. Can the output be expressed as a short linear combination of already-assigned
   values?
4. Is the operation repeated enough times that a local improvement will
   compound across the verifier?
5. Has this exact algebraic rewrite already been tried and ruled out in this repo?

If the answer is “yes” to questions 2 through 4, it is probably a good local
Midnight optimization candidate.

If yes for question 5, only retry it when you also have a concrete reason the generated
constraint shape will differ materially from the reverted version.
