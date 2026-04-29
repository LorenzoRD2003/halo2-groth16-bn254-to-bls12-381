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

- `execute-wrapper-direct-prove` still aborts with:
  `memory allocation of 268435456 bytes failed`
- that failure still appears even when the direct runtime is constrained to a
  single Rayon thread

This means:

- thread count is not the root cause
- removing `keygen_pk(...)` from the wrapper backend was necessary but not sufficient
- the current prove path still materializes too much prover-side state at once

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

Current accepted next step for the primary improvement:

- keep the richer setup artifact
- keep the local `midnight-proofs` patch
- treat the current `prove-trace` split as experimental and not yet reliable
- reduce eager coset materialization in the patched prover, especially around
  `compute_h_poly(...)`
- rework the split point only after the current first-stage trace path is made
  sound

## Current Failure State

The current first-stage split should still be treated as failing.

Latest valid observed status:

- `execute-wrapper-direct-prove-trace` fails with:
  `outer circuit input is not ready for synthesis: midnight create_proof_trace_from_base failed: The constraint system is not satisfied`
- the backend log reaches:
  - `prove-trace: validating setup verification key`
  - `prove-trace: using circuit_k=21`
  - `prove-trace: deserializing BaseProvingKey`
  - `prove-trace: entering create_proof_trace_from_base`
- and then fails before emitting a completed trace artifact

This means:

- the split path exists in code
- but it is not yet semantically correct
- it must be treated as experimental / broken until revalidated in a future pass
