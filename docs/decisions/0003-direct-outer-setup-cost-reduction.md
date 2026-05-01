# ADR 0003: Direct Outer Setup Cost Reduction

## Status

Accepted

## Context

The current direct outer backend setup path is too expensive in memory and
time, even for the small committed `circom_multiplier2` fixture.

Observed behavior:

- `execute-wrapper-direct-setup` can run for a long time and be terminated by
  the system under memory pressure.
- One measured run for `circom_multiplier2` was killed with peak RSS around
  `30.4 GB`.
- Splitting the CLI into `setup`, `prove`, and `verify` improves ergonomics and
  avoids recomputing setup across proofs, but it does not by itself reduce the
  peak memory of the setup phase.

Current implementation path:

- the direct backend setup/prove logic lives in
  [crates/wrapper-backends/src/outer/direct/proving.rs](../../crates/wrapper-backends/src/outer/direct/proving.rs)
- the heavy key generation comes from `midnight-proofs` `keygen_vk_with_k(...)`
  and especially `keygen_pk(...)`
- `midnight-proofs` `ProvingKey` stores multiple large representations at once,
  including:
  - `fixed_values`
  - `fixed_polys`
  - `fixed_cosets`
  - permutation proving-key structures
  - evaluator state

Relevant upstream source:

- `midnight-proofs-0.7.0/src/plonk/keygen.rs`
- `midnight-proofs-0.7.0/src/plonk/mod.rs`
- `midnight-proofs-0.7.0/src/plonk/permutation/keygen.rs`

This means the setup-time peak is dominated by the in-memory construction and
retention of duplicated polynomial representations inside the full proving key.

## Decision

We will pursue two setup-cost reductions, one primary and one secondary.

### Primary Improvement

Introduce a lean reusable setup artifact for the direct outer backend.

The lean setup artifact should persist only the minimum reusable data needed to
prove later, and should avoid persisting the full `ProvingKey` with every
precomputed representation already materialized.

Target direction:

- keep:
  - verification key
  - fixed-value data needed to reconstruct proving state
  - permutation base data needed to reconstruct proving state
  - circuit metadata such as `k`, backend, host lane, and public-input count
- do not persist:
  - `fixed_polys`
  - `fixed_cosets`
  - `l0`, `l_last`, `l_active_row`
  - evaluator caches or equivalent derived state

Instead:

- `setup` writes the lean artifact once
- `prove` reconstructs the derived proving structures lazily from that artifact
  before creating the proof

Rationale:

- this targets the actual source of setup memory pressure
- it preserves the desired UX of `setup once -> prove many -> verify many`
- it trades some extra proving-time recomputation for much lower setup-time
  memory cost

### Secondary Improvement

Cache universal-ish KZG params by `(outer_host, k)`.

Target direction:

- generate params once per host lane and circuit size `k`
- persist or reuse them across `setup`, `prove`, and `verify`

Rationale:

- this does not appear to be the dominant source of the setup memory spike
- but it still removes repeated work from the direct backend flow
- it makes the separation between:
  - universal-ish params
  - circuit-specific key material
  clearer in both code and CLI behavior

## Consequences

Expected positive consequences:

- lower peak memory in `setup`
- smaller persisted setup artifacts
- better chance that local developer machines can complete setup
- clearer lifecycle separation:
  - universal params
  - circuit-specific setup artifact
  - proof generation
  - verification

Expected tradeoffs:

- `prove` becomes responsible for reconstructing some derived state
- proving may become somewhat slower than using a fully materialized persisted
  `ProvingKey`
- backend complexity increases because setup/prove artifacts are no longer just
  “serialize everything”

## Non-Decision

This ADR does not yet choose:

- the exact serialized format of the lean setup artifact
- whether the lean artifact belongs only in this repository or should be
  proposed upstream to `midnight-proofs`
- whether params caching should be in-memory only, filesystem-backed, or both

## Next Step

Implement the primary improvement first:

1. define a lean direct-setup artifact format
2. teach `execute-wrapper-direct-setup` to emit it
3. teach `execute-wrapper-direct-prove` to reconstruct derived state from it
4. remeasure peak RSS and wall-clock time on `circom_multiplier2`

Implement the secondary improvement after that:

1. cache params by `(outer_host, k)`
2. thread cached params through setup/prove/verify
3. measure whether it materially changes runtime

## Current Result

The first version of the primary improvement has now been implemented in the
repository, and its limitations are now understood.

Current behavior:

- `execute-wrapper-direct-setup` emits:
  - a setup bundle JSON
  - a proving-key sidecar file
- the repository now also exposes a split prove path:
  - `execute-wrapper-direct-prove-trace`
  - `execute-wrapper-direct-prove-finalize`
- artifact hygiene rule for this split:
  - if setup-producing code changes, old setup artifacts should be deleted
    before later prove/finalize measurements are trusted
  - if trace-producing code or trace serialization changes, old trace artifacts
    and trace logs should be deleted before rerunning
  - if finalize-producing code or finalized proof-bundle shape changes, old
    finalized proof artifacts and finalize logs should be deleted before
    rerunning
- the setup bundle persists verification materials and metadata
- the proving-key sidecar persists:
  - verification key
  - fixed values in Lagrange form
  - permutation base data
- `execute-wrapper-direct-prove` now reuses:
  - `BaseProvingKey::read(...)`
  - `create_proof_from_base(...)`
- direct execution commands now apply a hard per-process memory limit of
  `24 GiB`

Observed result on the committed `circom_multiplier2` fixture:

- one successful lean setup run produced:
  - `circuit_k = 21`
  - `public_input_count = 1`
  - `setup_elapsed_ms = 1554572`
- `1554572 ms` is approximately `25m 54s`

However, the current richer prove path still fails in practice:

- `execute-wrapper-direct-prove-finalize` still aborts with:
  `memory allocation of 268435456 bytes failed`
- the split `execute-wrapper-direct-prove-trace` stage now succeeds, so the
  failure is isolated to the finalization half of the split
- that failure still appears even after removing the fixed 4-thread Rayon cap

This means:

- thread count is not the root cause
- removing `keygen_pk(...)` from the wrapper backend was necessary but not sufficient
- the current finalization path still materializes too much prover-side state at once

Refined diagnosis:

- the repository now avoids rerunning `keygen_pk(...)` in the wrapper backend
- the next remaining hotspot is inside the prover itself
- the current most suspicious site is eager extended-domain coset
  materialization inside `compute_h_poly(...)`, especially:
  - `advice_cosets`
  - `instance_cosets`
- the observed failing allocation size:
  - `268435456` bytes
  - matches a `256 MiB` block
  - which is consistent with one large extended-domain polynomial allocation
    for `k = 21`

So the current implementation successfully made setup richer and removed
`keygen_pk(...)` from the wrapper-side prove path, but it has not yet reduced
the eager prover-side coset allocation pattern.

One retained follow-on mitigation is now also in place:

- the persisted split trace stores advice/instance witness polynomials in
  coefficient form instead of Lagrange form
- split finalization now uses a reduced `OpeningKey` that carries only:
  - fixed coefficient-form polynomials
  - permutation coefficient-form polynomials
  - verification-key metadata needed for transcript/opening work
- both retained split halves now treat fixed columns sparsely:
  - `HPolyKey` materializes only fixed cosets actually used by
    `compute_h_poly(...)`
  - `OpeningKey` materializes only fixed coefficient-form polynomials actually
    used by transcript evaluations and opening queries
- the retained `compute_h_poly(...)` path now also avoids eager permutation
  sigma-coset materialization:
  - `HPolyKey` keeps permutation polynomials in Lagrange form
  - `evaluate_h(...)` derives the needed sigma cosets lazily per permutation chunk
- the permutation chunk size is now exposed through
  `execute-wrapper-direct-prove-finalize --h-poly-row-chunk-size ...`
  so memory/runtime tradeoffs can be calibrated experimentally from the CLI
- that CLI flag is intentionally opt-in and accepts a base-2 exponent instead
  of a raw row count
- the heavier coset state remains on the pre-`compute_h_poly` side of the split

That removes one whole class of simultaneous witness duplication during
`prove-finalize`, but the remaining peak is still dominated by the
pre-`compute_h_poly` prover state.

Current accepted next step for the primary improvement:

- keep the richer setup artifact
- keep the local `midnight-proofs` patch
- keep the split semantically correct and instrumented
- use the new finalize checkpoints to identify the exact last successful
  post-`compute_h_poly` subphase before an OOM abort
- use the finer-grained `finalize_for_h_poly()` checkpoints to distinguish:
  - `before compute_lagrange_polys`
  - `after compute_lagrange_polys`
  - `before sparse fixed cosets`
  - `after sparse fixed cosets`
  - `before permutation h key`
  - `after permutation h key`
- continue reducing memory either in the pre-`compute_h_poly` persisted state
  or in the remaining post-`compute_h_poly` opening/query work based on that log evidence

## Current Failure State

The current first-stage split is now working, but split finalization remains
memory-bound.

Latest valid observed status:

- `execute-wrapper-direct-prove-trace` succeeds and emits a completed trace artifact
- under the previous split position, `execute-wrapper-direct-prove-finalize` aborted with:
  `memory allocation of 268435456 bytes failed`
- experiments that moved `h_poly` into `prove-trace` were not retained because
  they pushed the memory spike back into `prove-trace`
- the current retained format therefore still splits before `compute_h_poly(...)`
  and requires a fresh trace artifact for the current retained format

This means:

- the split path exists in code
- the trace half is semantically correct
- the current next action is to rerun the split flow and use the finer-grained
  finalize logs to identify which retained subphase still owns the memory spike
- those reruns should always start from artifacts regenerated after the latest
  relevant code change, not from older setup/trace/finalize outputs
