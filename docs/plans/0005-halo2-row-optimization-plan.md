# Halo2 Row Optimization Plan

## Purpose

This document turns the current row-optimization investigation into an
implementation plan for the BN254 pairing-core lane.

Primary goal:

- reduce real layout rows in the current Groth16-relevant pairing-core path

Primary measured targets:

- `bn254_final_exponentiation_hard_part`
- `bn254_final_exponentiation`
- `bn254_pairing_check_groth16_style`
- `groth16_fixture_verifier_total`

This is an implementation plan, not an ADR. It should guide incremental work
that stays inside the current Stage 1 / Week 5+ scope.

## Current Baseline

The repository's current retained optimization baseline says:

- the dominant hotspot is still `final_exponentiation_hard_part`
- the strongest retained win so far was compressed cyclotomic squaring inside
  `exp_by_neg_x(...)`
- the current verifier-shaped pairing path already benefits from prepared
  constant G2 verifier-key terms

Relevant retained measurements from existing docs:

- `final exponentiation hard part`: `561254 -> 492083`
- `final exponentiation`: `574562 -> 505391`
- `pairing check` Groth16-style: `1936380 -> 1867209`

See:

- `docs/profiling.md`
- `docs/midnight-local-optimization-notes.md`
- `docs/decisions/0002-bn254-local-optimization-policy.md`

## Guiding Constraints

- optimize for measured rows first
- stay on the current Midnight-backed BN254 primitive path
- avoid speculative broad API work
- do not re-land already rejected `linear_combination(...)` rewrites without a
  materially different constraint shape
- do not treat selector cleanup or boolean equality refactors as row
  optimizations unless profiling proves otherwise

## Highest-Value Opportunities

### 0. Search for a better `exp_by_neg_x(...)` chain automatically

Current issue:

- `exp_by_neg_x(...)` is still the dominant repeated local hotspot inside the
  hard part
- the retained signed-window chain is strong, but it was still found by a
  manual design pass rather than a systematic search over the real cost model

Why this is now high priority:

- Phases 1, 3, 4, and 5 all failed to beat the retained baseline
- the strongest wins so far came from chain-level scheduling and compressed
  square-block treatment, not from local formula cleanup
- this is the cleanest remaining line that changes cost without reusing the
  same torus/Frobenius/sum-diff families that already plateaued

Proposed direction:

- search over alternate precomputed odd windows, starting seeds, and signed
  step schedules that still reconstruct `BN254_X_ABS`
- rank candidates first with a cost proxy that distinguishes:
  compressed square blocks,
  positive window multiplies,
  negative window multiplies through conjugation,
  and precomputation cost
- only then bring the best candidates into measured circuit profiling

Primary files:

- `crates/wrapper-circuits/src/bn254/final_exp_chain.rs`
- `crates/wrapper-circuits/src/bn254/host/pairing_host.rs`
- `crates/wrapper-circuits/src/bn254/g2/miller.rs`
- `docs/profiling.md`

### 1. Replace witness-only Frobenius materialization with circuit-native transforms

Current issue:

- `AssignedFp12::frobenius_map(...)` currently computes the result on the host
  and reassigns it as a witness
- this likely pays unnecessary assignment and equality cost for a transform
  whose algebra is fixed and mostly linear over already-assigned coordinates

Why this is promising:

- the repo already shows that `mul_by_constant(...)` is the strongest local
  primitive on the current foreign-field path
- BN254 Frobenius in `Fp2`, `Fp6`, and `Fp12` decomposes into conjugations,
  sign flips, and fixed-coefficient multiplies
- the hard part uses `frobenius_map(...)` multiple times in a row-sensitive
  region

Primary files:

- `crates/wrapper-circuits/src/bn254/host/mod.rs`
- `crates/wrapper-circuits/src/bn254/fp6.rs`
- `crates/wrapper-circuits/src/bn254/fp12.rs`
- `crates/wrapper-circuits/src/bn254/g2/miller.rs`

### 2. Fuse `frobenius(x) * y` sites in the hard part

Current issue:

- the hard part currently materializes Frobenius outputs and then separately
  multiplies them

Why this is promising:

- the row-sensitive region in `final_exponentiation_hard_part(...)` has three
  such sites
- if Phase 1 exposes cheaper circuit-native Frobenius transforms, a second pass
  can try to avoid materializing those intermediates entirely

Primary files:

- `crates/wrapper-circuits/src/bn254/fp12.rs`
- `crates/wrapper-circuits/src/bn254/g2/miller.rs`

### 3. Fuse `sum/diff + mul` patterns in the Fp12 hot path

Current issue:

- `mul_with_precomputed_sums(...)` and
  `mul_by_unitary_inverse_with_precomputed_sums(...)` save work relative to the
  generic path, but call sites still materialize many `sum_components(...)` and
  `diff_components(...)` values as separate steps

Why this is promising:

- `exp_by_neg_x(...)` and the hard part make this pattern pervasive
- this is a better fit for Halo2-style local gate fusion than broader algebraic
  rewrites that already lost

Primary files:

- `crates/wrapper-circuits/src/bn254/fp12.rs`
- `crates/wrapper-circuits/src/bn254/g2/miller.rs`

### 4. Revisit torus / compressed-region work only as a longer region

Current issue:

- isolated torus substitutions already lost

Why it is still listed:

- a longer compressed run may still amortize compression and decompression
- this remains the highest-risk structural follow-up if Phases 1 and 2 stall

Status:

- explicitly deferred until after the lower-risk phases below

### 5. Redesign the subgroup multiplication kernel, not just its packaging

Current issue:

- the repo already disproved two families of "same arithmetic, different
  wrapper" changes:
  isolated torus substitution and chip-style subgroup multiply repackaging

Why this remains open:

- the hard part still spends most of its cost in cyclotomic-subgroup
  multiplication families
- what failed was not "all subgroup-aware kernels", but specifically kernels
  that still paid the old ambient arithmetic costs after compression,
  decompression, or repackaging overhead

Promising direction:

- target a genuinely different multiply formula for one of:
  `cyclotomic * cyclotomic`,
  `cyclotomic * unitary_inverse(cyclotomic)`,
  or the precomputed-window multiplies inside `exp_by_neg_x(...)`
- make the algebraic objective explicit in terms of reducing real `Fp6` / `Fp2`
  multiplies, not in terms of changing code structure

Primary files:

- `crates/wrapper-circuits/src/bn254/fp12.rs`
- `crates/wrapper-circuits/src/bn254/g2/miller.rs`
- `docs/plans/0002-cyclotomic-unitary-kernel-design.md`

## Implementation Phases

## Phase 1. Circuit-native Frobenius path

Goal:

- remove witness-only `Fp12` Frobenius materialization from the hot path

Hypothesis:

- re-expressing `Fp6` and `Fp12` Frobenius as circuit-native transforms built
  from existing `Fp2` conjugation/sign-flip logic plus `mul_by_constant(...)`
  will reduce rows in the hard part and pairing totals

Scope:

- add explicit circuit-side Frobenius helpers for `Fp6`
- rewrite `AssignedFp12::frobenius_map(...)` to compose those helpers rather
  than assigning a host-computed witness
- keep host/reference helpers unchanged as the semantic oracle
- update the hard part to consume the new implementation without changing its
  formula structure yet

Files expected to change:

- `crates/wrapper-circuits/src/bn254/fp6.rs`
- `crates/wrapper-circuits/src/bn254/fp12.rs`
- `crates/wrapper-circuits/src/bn254/g2/miller.rs`
- `crates/wrapper-circuits/src/bn254/tests/field_and_tower.rs`
- possibly `crates/wrapper-circuits/src/bn254/tests/pairing.rs`

Suggested implementation order:

1. Add `AssignedFp6::frobenius_map(...)` using circuit-native `Fp2`
   Frobenius plus fixed `Fp6` Frobenius coefficients.
2. Rewrite `AssignedFp12::frobenius_map(...)` to use the `Fp6` helper plus the
   fixed `Fp12` `c1` coefficient multiply.
3. Keep the host/reference formulas as-is and add direct tests that compare the
   new circuit-side transform against the current host-side constants.
4. Re-run the narrow block metrics before touching any other hot-path formula.

Acceptance criteria:

- existing correctness tests still pass
- new Frobenius-specific tests pass on randomized samples
- `profile-layout --family blocks` shows no regression in:
  `bn254_final_exponentiation_hard_part`,
  `bn254_final_exponentiation`,
  `bn254_pairing_check_groth16_style`
- if rows improve or remain flat on blocks, proceed to a verifier-level check
  on `groth16_fixture_verifier_total`

Stop / revert condition:

- any regression in `bn254_final_exponentiation_hard_part` large enough to
  outweigh flat or tiny wins elsewhere

Phase 1 non-goals:

- no torus-region work
- no `linear_combination(...)` revisit
- no broad API redesign for `Fp12`
- no floor-planner experiments yet

## Phase 2. Fused Frobenius-multiply sites

Goal:

- remove materialized intermediates at the three `frobenius(...) * cyclotomic`
  sites in the hard part

Expected targets:

- `y12 = frobenius(y9, 1) * y11`
- `y14 = frobenius(y8, 2) * y12`
- `y15 = frobenius(y9 * unitary_inverse(r), 3) * y14`

Idea:

- add one or more narrow helpers that combine Frobenius with the existing
  `mul_with_precomputed_sums(...)` path so the transformed intermediate does not
  need to be assigned as a separate full `Fp12`

Risk:

- medium; this is still algebraically straightforward, but it starts coupling
  transform and product logic

## Phase 3. Fused sum/diff hot-path helpers

Goal:

- reduce repeated `sum_components(...)` / `diff_components(...)` materialization
  inside `exp_by_neg_x(...)` and `final_exponentiation_hard_part(...)`

Idea:

- introduce narrower helper variants that accept the original operands and
  compute the needed sum/diff closer to the multiply site, or that compute the
  full product with fewer externally materialized intermediates

Risk:

- medium to high; this needs careful measurement because "more fused" is not
  automatically cheaper on Midnight's foreign-field stack

## Phase 4. Layout / floor-planning experiments

Goal:

- test whether region packing is leaving measurable row slack on the current
  primitive circuits

Idea:

- audit how much of the measured row count appears to come from many small
  regions under `SimpleFloorPlanner`
- only pursue this if Phases 1 to 3 plateau

Risk:

- medium; this may require broader structural changes for limited upside

## Phase 5. Long compressed-region prototype

Goal:

- revisit torus/compressed cyclotomic arithmetic only if we are willing to keep
  a meaningful hard-part run in compressed form

Entry condition:

- only after Phases 1 to 4 are measured and either landed or rejected

Risk:

- highest

## Phase 6. Automated `exp_by_neg_x(...)` Chain Search

Goal:

- find a better fixed chain for `BN254_X_ABS` than the current retained
  signed-window schedule, using a search driven by the repo's actual operation
  cost asymmetries

Scope:

- keep the existing correctness contract and fixed exponent target
- search over:
  starting odd window,
  window set,
  signed step sequence,
  square-block lengths
- produce candidates that can be encoded into `final_exp_chain.rs`

First implementation slice:

1. add a host-side search harness that reconstructs `BN254_X_ABS`
2. define an explicit cost proxy with separate weights for:
   compressed square blocks,
   cyclotomic-subgroup multiplies,
   unitary-inverse multiplies,
   and precomputation windows
3. emit the best few chains plus their reconstructed exponent and proxy score
4. profile only the top candidates in the real circuit path

Acceptance criteria:

- at least one candidate that is circuit-profiled against the retained chain
- no retention without a measured `blocks` win

Risk:

- medium; the search space can grow quickly, but the direction is still much
  cheaper than broad arithmetic refactors

## Phase 6 Early Result

The first implementation slice for Phase 6 now exists:

- a host-side search harness for `BN254_X_ABS`
- a proxy scoring model that separates:
  compressed square blocks,
  positive window multiplies,
  negative window multiplies,
  and distinct-window precomputation cost
- a `wrapper-cli search-exp-by-x-chain` command that prints the retained chain
  and the top proxy-ranked candidates

With the current default proxy weights:

- retained chain proxy score: `162`
- top discovered candidate proxy score: `153`

One current top candidate is:

```text
69 <<10,-83 <<4,-107 <<8,-83 <<10,+75 <<7,-75 <<8,-123 <<9,-15
```

Interpretation:

- the search harness is operational and does find candidates that beat the
  retained chain under the current proxy
- the next required step is not more search machinery, but profiling one or two
  top candidates in the real circuit path before trusting the proxy

Follow-up result with retained-window restriction:

- restricting the search to the current retained window family
  `{17, 35, 37, 79, 83, 101, 105}`
  leaves the retained schedule as the best proxy-scored candidate
- the next-ranked candidates in that restricted family all score worse than the
  retained baseline

Interpretation:

- if a better chain exists, it likely needs either:
  a different precomputed window family,
  a better precompute-cost model,
  or both
- this makes the unrestricted search results more interesting, but also raises
  the bar for circuit profiling: any candidate with a new window family must
  justify its extra precompute footprint with a real measured `blocks` win

Follow-up result with empirical proxy weights:

- the search command now supports an `empirical` weight profile driven by real
  circuit rows for:
  compressed cyclotomic square blocks,
  generic `Fp12` multiplication,
  and `mul_by_unitary_inverse`
- with that empirical profile and the retained-window restriction, the retained
  schedule is still the best-ranked candidate

Interpretation:

- both the linear proxy and the empirical proxy now agree that the current
  retained window family does not contain a better schedule than the baseline
- the most promising next profiling targets are therefore unrestricted
  candidates that introduce new window families, not small rearrangements of
  the current retained family

## Phase 7. New Multiplication Kernel Investigation

Goal:

- design a subgroup-aware multiplication kernel that changes the real operation
  count, not just the representation or wrapper around the current formula

Scope:

- start host/reference-only
- quantify the ambient cost in terms of underlying `Fp6` / `Fp2` multiplies
- require a concrete hypothesis for where the operation count drops before any
  circuit prototype is attempted

Entry condition:

- after at least one Phase 6 chain-search pass completes, or sooner if a
  concrete new kernel proposal emerges

Risk:

- highest

## Measurement Workflow

Before each phase:

```bash
cargo run -q -p wrapper-cli -- doctor
cargo run -q -p wrapper-cli -- profile-layout --family blocks > /tmp/before.tsv
```

After each phase:

```bash
cargo run -q -p wrapper-cli -- doctor
cargo run -q -p wrapper-cli -- profile-layout --family blocks > /tmp/after.tsv
```

Verifier-level follow-up when block results are promising:

```bash
cargo run -q -p wrapper-cli -- profile-layout --family groth16
```

Primary rows to diff first:

- `bn254_final_exponentiation_hard_part`
- `bn254_final_exponentiation`
- `bn254_pairing_check_groth16_style`

Secondary rows to watch:

- `bn254_miller_loop_narrow`
- `groth16_fixture_verifier_total`
- `groth16_pairing_check_proxy_4_terms`

## Phase 1 Ready State

The repository is ready to begin Phase 1 now.

Immediate first implementation slice:

1. inspect the current `Fp6` and `Fp12` Frobenius coefficient helpers in
   `crates/wrapper-circuits/src/bn254/host/mod.rs`
2. add circuit-native `AssignedFp6::frobenius_map(...)`
3. rewrite `AssignedFp12::frobenius_map(...)`
4. add narrow randomized correctness coverage
5. remeasure `blocks`

If Phase 1 wins:

- land it as an isolated optimization PR
- then start Phase 2 from the new retained baseline

If Phase 1 is neutral:

- decide whether the code is simpler or more obviously correct
- otherwise revert and move directly to Phase 2 experiments on a throwaway
  branch

If Phase 1 loses:

- revert cleanly
- record the result in `docs/midnight-local-optimization-notes.md`
- move to the next phase without stacking the regression

## Phase 1 Attempt Result

The first Phase 1 implementation attempt was:

- add circuit-native `AssignedFp6::frobenius_map(...)`
- rewrite `AssignedFp12::frobenius_map(...)` to use circuit arithmetic instead
  of host-computed witness assignment

Correctness result:

- targeted Frobenius tests passed
- broader `field_and_tower` tests passed

Measured `profile-layout --family blocks` result relative to the retained
baseline:

- `bn254_miller_loop_narrow`: unchanged at `457060`
- `bn254_final_exponentiation_easy_part`: `12288 -> 12630`
- `bn254_final_exponentiation_hard_part`: `492083 -> 494953`
- `bn254_final_exponentiation`: `505391 -> 508603`
- `bn254_pairing_check_groth16_style`: `1867209 -> 1870421`
- `bn254_pairing_check_sample_2_terms`: `1600495 -> 1603707`

Decision:

- reverted

Interpretation:

- on the current Midnight foreign-field stack, witness-driven Frobenius
  materialization is still cheaper than replacing it with explicit circuit-side
  conjugation plus fixed-coefficient multiplication
- future follow-up should therefore focus on later-phase opportunities such as
  fused Frobenius-multiply sites or other hot-path arithmetic reductions, not a
  direct replacement of `AssignedFp12::frobenius_map(...)`

## Phase 2 Attempt Result

The first Phase 2 implementation attempt was:

- add a fused `AssignedFp12` helper that multiplies
  `frobenius(self, power)` by a cyclotomic `rhs` while keeping the current
  witness-driven Frobenius boundary internal to the multiplication site
- replace the three hard-part sites:
  `frobenius(y9, 1) * y11`,
  `frobenius(y8, 2) * y12`,
  `frobenius(y15, 3) * y14`
  with the fused helper

Correctness result:

- targeted fused-helper tests passed
- broader `field_and_tower` tests passed
- `final_exponentiation_matches_arkworks_on_generator_miller_output` passed

Measured `profile-layout --family blocks` result relative to the retained
baseline:

- `bn254_miller_loop_narrow`: unchanged at `457060`
- `bn254_final_exponentiation_easy_part`: unchanged at `12288`
- `bn254_final_exponentiation_hard_part`: `492083 -> 491633`
- `bn254_final_exponentiation`: `505391 -> 504941`
- `bn254_pairing_check_groth16_style`: `1867209 -> 1866759`
- `bn254_pairing_check_sample_2_terms`: `1600495 -> 1600045`

Decision:

- retained

Interpretation:

- the explicit circuit-native Frobenius rewrite was too expensive, but the
  narrower fused-consumption rewrite does recover a small real row win
- the best next follow-up remains local hot-path fusion, not broader
  circuit-native Frobenius replacement

## Phase 3 Attempt Result

The first Phase 3 implementation attempt was:

- add partial `Fp12` helpers that internalize one-use `sum_components(...)` and
  `diff_components(...)` values close to the multiply sites
- apply those helpers in `exp_by_neg_x(...)` and
  `final_exponentiation_hard_part(...)`

Correctness result:

- the first draft had a caller-wiring bug and was fixed before evaluation
- after the fix, `field_and_tower` tests passed
- `final_exponentiation_matches_arkworks_on_generator_miller_output` passed

Measured `profile-layout --family blocks` result relative to the retained
Phase 2 baseline:

- `bn254_miller_loop_narrow`: unchanged at `457060`
- `bn254_final_exponentiation_easy_part`: unchanged at `12288`
- `bn254_final_exponentiation_hard_part`: unchanged at `491633`
- `bn254_final_exponentiation`: unchanged at `504941`
- `bn254_pairing_check_groth16_style`: unchanged at `1866759`
- `bn254_pairing_check_sample_2_terms`: unchanged at `1600045`

Decision:

- reverted

Interpretation:

- moving one-use `sum/diff` materialization closer to the call site did not
  buy rows on the current Midnight foreign-field stack
- keeping the simpler retained Phase 2 code is preferable to carrying neutral
  fusion helpers

## Phase 4 Attempt Result

The Phase 4 implementation work added:

- a generic native-circuit wrapper that can override the floor planner with
  `midnight_proofs::circuit::floor_planner::V1`
- `*_v1_layout_metrics()` measurement helpers for the current pairing-core
  block circuits
- a `profile-layout --family floor-planner` CLI family intended to compare the
  default `SimpleFloorPlanner` against `V1`
- an ignored local probe test for representative block metrics

Compilation result:

- `cargo check -p wrapper-circuits` passed
- `cargo check -p wrapper-cli` passed

Practical measurement result:

- full `floor-planner` CLI comparisons were too slow to complete within a
  reasonable local iteration budget, even after reducing the family to the two
  most relevant hotspots
- the narrower ignored probe over
  `final_exponentiation_hard_part` and
  `pairing_check_groth16_style`
  was also too slow to return actionable numbers in a reasonable local run

Decision:

- no planner switch retained
- keep the comparison tooling available, but do not treat `V1` as a practical
  active optimization path for the current repo until someone is willing to run
  longer dedicated measurements

Interpretation:

- there is not yet evidence that floor-planner changes will produce a better
  rows-per-engineering-time tradeoff than the retained arithmetic-level work
- Phase 4 does not currently beat the simpler strategy of continuing with
  arithmetic or representation-level optimizations

## Phase 5 Attempt Result

The Phase 5 implementation attempt explored a longer torus/compressed region in
the hard part:

- keep the `y7 -> y8` middle run in a torus-style compressed representation
- decompress only when returning to the `y9` / `y10` branch of the retained
  hard-part schedule

What was validated:

- a host/reference prototype for the `y7 -> y8` torus-middle region matched the
  retained hard-part baseline on generator and randomized easy-part samples
- a circuit prototype for the same torus-middle region matched the host-side
  expected output

Measured result:

- `final_exponentiation_hard_part` rows regressed from `491633` to `499018`

Decision:

- reverted

Interpretation:

- even after amortizing compression across a longer `y7 -> y8` region, the
  decompress and generic Fp6 inversion costs still outweighed the savings
- this confirms the repo's earlier evidence that a torus-style direction needs
  a genuinely cheaper subgroup kernel, not just a longer run of the current
  formulas
