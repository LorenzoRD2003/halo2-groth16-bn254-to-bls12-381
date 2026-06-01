# ADR 0004: Local Patch for `midnight-proofs`

## Status

Accepted

## Context

The repository needs a more reusable direct outer setup artifact, but the
required proving-key decomposition is not available through the public API of
the upstream `midnight-proofs` crate.

The current problem is:

- `execute-wrapper-direct-setup` and `execute-wrapper-direct-prove` need a
  richer split between setup-time and prove-time state
- the correct split should avoid rerunning full `keygen_pk(...)` during `prove`
- but the proving-key internals required for that split are not exposed by
  upstream `midnight-proofs`

Specifically, the setup/prove design needs access to:

- a lean proving-state setup artifact that stores:
  - verification key
  - fixed values in Lagrange form
  - permutation base data
- a way to reconstruct the derived proving caches from that artifact:
  - fixed polys
  - fixed cosets
  - permutation proving data
  - `l0`, `l_last`, `l_active_row`
  - evaluator state

Upstream `midnight-proofs` currently exposes only:

- `keygen_vk_with_k(...)`
- `keygen_pk(...)`
- full `ProvingKey`

and most of the pieces needed for a richer split are internal or `pub(crate)`.

## Decision

We will carry a local workspace patch of `midnight-proofs` for now.

The patch is wired through:

- `[patch.crates-io]` in the workspace [Cargo.toml](../../Cargo.toml)
- local crate path:
  [patches/midnight-proofs](../../patches/midnight-proofs)
- root-workspace exclusion:
  `exclude = ["patches/midnight-proofs"]`
  so `cargo` commands can also be run directly inside the patch directory

The local patch adds a new internal/public split that the wrapper repo can use:

- `plonk::BaseProvingKey`
  - a lean proving-state setup artifact
  - stores:
    - `vk`
    - `fixed_values`
    - permutation base data
  - omits:
    - `fixed_polys`
    - `fixed_cosets`
    - `l0`, `l_last`, `l_active_row`
    - evaluator caches
- `plonk::keygen_pk_base(...)`
  - generates the lean setup artifact without constructing the full final
    proving key object
- `create_proof_from_base(...)`
  - creates a proof directly from `BaseProvingKey`
  - avoids rerunning full `keygen_pk(...)` in the wrapper backend

The patch also introduces:

- `permutation::BaseProvingKey`
  - lean permutation proving-state setup artifact
  - stores only permutation polynomials in Lagrange form
  - reconstructs coefficient/coset forms lazily on finalize
- `PersistedProverTrace`
  - persisted first-stage artifact for the split direct-proving lane
  - now serializes:
    - one prepared finalization section carrying the pre-`compute_h_poly`
      coset/materialized prover state
    - one coefficient-form opening/query section
  - keeps advice/instance witness polynomials in coefficient form instead of
    Lagrange form inside the persisted trace payload
- `OpeningKey`
  - derived proving state used only after `h_poly` has already been computed
  - owns only the fixed/permutation coefficient-form polynomials needed for
    transcript evaluations and multi-opening queries
  - now materializes only fixed columns actually referenced by `fixed_queries`
- `HPolyKey`
  - now avoids eager permutation sigma-coset materialization
  - retains permutation polynomials in Lagrange form and lets the evaluator
    derive sigma cosets lazily per permutation chunk
  - also no longer requires cloning one second full `BaseProvingKey` before
    finalization experiments
- `Evaluator::custom_gates`
  - now keeps one `GraphEvaluator` per real custom gate instead of one
    monolithic evaluator over every gate polynomial in the circuit
  - this lets `evaluate_h(...)` materialize and drop fixed columns gate by
    gate during `custom_fixed_cosets`, reducing the worst peak-memory batch in
    BLS12-hosted finalize runs

Current state of the patch:

- it is active through `[patch.crates-io]`
- it compiles in the workspace
- it now passes `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  from the repository root
- it now also passes
  `cargo clippy --all-targets --all-features -- -D warnings`
  when run directly from `patches/midnight-proofs/`
- the direct outer backend now uses:
  - `keygen_pk_base(...)` during setup
  - `BaseProvingKey::read(...)` during prove
  - `create_proof_from_base(...)` during prove
  - `create_proof_trace_from_base(...)` for the first stage of prove
  - `finalise_proof_from_base_trace(...)` for the second stage of prove
- the patch-maintenance delta now also includes:
  - a `dev-curves` feature mapping that forwards to `midnight-curves/dev-curves`
  - benchmark-support updates under `src/plonk/bench/prover.rs` so the internal
    prover benchmark code still matches the new `ProverTrace` / finalization split
  - a local placeholder `benches/zswap_output.rs` because the original upstream
    benchmark depends on downstream Midnight application crates that bring in a
    second, incompatible `midnight-proofs` instance when compiled in this repo
  - sparse fixed-column handling inside `evaluate_h(...)`
- lazy per-chunk permutation sigma-coset derivation inside `evaluate_h(...)`
- CLI-configurable row chunk sizing for the chunked `h_poly` permutation path
  through an opt-in base-2 exponent flag

## Why this is done here

This repository needs to iterate on the direct outer setup/prove split now.

Waiting for upstream changes would block:

- setup/prove/verify split validation
- memory-cost experiments
- application-shaped integration work such as Semaphore and ZK Email

The patch is therefore intentionally local-first:

- prove the design here
- measure the effect here
- upstream later if the direction proves correct

## What this is trying to solve

The patch is intended to solve the gap between:

- a setup artifact that is too lean to be useful
- and a full proving key that is too expensive to recompute or persist naively

More concretely, the goal is:

- `setup once`
- persist a richer but still lean artifact
- `prove many` without rerunning full `keygen_pk(...)`

This should move the split toward:

- setup-time work:
  - circuit synthesis
  - fixed-value extraction
  - permutation base extraction
  - VK generation
- prove-time work:
  - derive only the caches needed to create a proof

## Consequences

Positive:

- unblocks experimentation without waiting on upstream
- makes the setup/prove split expressible in this repository
- provides a concrete shape for future upstreaming

Negative:

- the workspace now carries a local fork of a cryptographic dependency
- future upgrades of `midnight-proofs` will require patch maintenance
- there is now divergence risk between local behavior and upstream behavior
- a small amount of non-prover upstream benchmark surface is intentionally
  neutralized locally when it cannot be made to share this repository's patched
  `midnight-proofs` instance honestly

## Boundaries

This ADR does not claim that the patch fully solves the setup/prove cost issue.

It only establishes that:

- the richer split requires local `midnight-proofs` support
- that support now lives in this repository under `patches/midnight-proofs`

Further measurement is still required to know whether the patched design is
enough or whether more aggressive cost reduction is necessary.

## Current Limitation

Although the patch now avoids rerunning `keygen_pk(...)`, it does not yet fully
solve the prove-time memory spike.

The patch now reduces two layers of prover-side duplication:

- `PersistedProverTrace` no longer persists advice/instance witness polynomials
  in Lagrange form
- split finalization now uses one smaller `OpeningKey` for:
  - fixed coefficient-form polynomials
  - permutation coefficient-form polynomials
  - transcript/opening-query work
- the coset-heavy proving state remains on the pre-`compute_h_poly` side of the
  split, where it can be persisted and diagnosed independently
- the `compute_h_poly(...)` path now also treats fixed columns sparsely:
  - `HPolyKey.fixed_cosets` materializes only fixed columns actually used by
    the evaluator/permutation path
  - `evaluate_h(...)` now accepts sparse fixed columns in the same style as the
    already-sparse advice/instance columns

The patch also now supports a practical experimentation split:

- compute and persist the first-stage prover trace before `compute_h_poly(...)`
- finalize from that persisted trace later

## Transcript Continuation Bug and Fix

While validating the split `create_proof_trace_from_base(...)` /
`finalise_proof_from_base_trace(...)` lane against direct verification, the
repository hit a proof-transcript divergence that affected both the BN254-hosted
and BLS12-hosted direct backends.

The root cause was in transcript restoration for split finalization:

- `PersistedProverTrace` persisted only the already-emitted proof bytes
- `PreparedFinalizationTrace::init_transcript(...)` reconstructed the transcript
  with `T::init_from_bytes(prefix)`
- that recreated only the transcript buffer, not the Fiat-Shamir hash state
- and it also positioned the cursor at the start of the proof bytes instead of
  the append boundary

This meant the split lane diverged immediately when finalization resumed:

- the first post-trace `transcript.write(...)` appended with the wrong internal
  challenge state
- and before the fix it also overwrote the proof prefix at offset `0`
- the verifier then failed very early with
  `Transcript error: Invalid ... point encoding in proof`

The local patch now fixes that by:

- introducing `ReplayableCircuitTranscript`
- persisting a replay log of transcript `common(...)` absorptions and challenge
  squeezes alongside the already-emitted proof bytes
- restoring the transcript by replaying those operations into a fresh
  Blake2b state
- and reopening the proof buffer at the end so split finalization appends
  instead of overwriting

One important detail from the regression work:

- the split proof is not expected to be byte-for-byte identical to the
  monolithic proof after the continuation point
- `vanishing.construct(...)` blinds quotient limbs with fresh randomness during
  the final stage, so later proof bytes can legitimately differ
- the correct regression target is therefore:
  - the preserved proof prefix from the pre-finalization stage
  - plus successful verifier preparation / verification of the final split proof

## Sparse Lookup Fixed-Column Regression

After the transcript continuation bug was fixed, direct verification stopped
failing on transcript parsing and started failing later with `OpeningError`.

The smaller `tests/plonk_api.rs` regression isolated one concrete remaining
cause inside `evaluate_h(...)`:

- the sparse fixed-column selection for the lookup section was incomplete
- materializing only the columns reported by the lookup evaluator could omit
  fixed columns that the full lookup argument still needed during `h(X)`
  construction
- that produced a structurally valid proof with inconsistent opening claims,
  which later surfaced as verifier-side `OpeningError`

The retained correctness fix is:

- keep sparse lookup fixed-column materialization
- but complement the `GraphEvaluator`-derived lookup set by walking
  `lookup.input_expressions()` and `lookup.table_expressions()` directly
- include any fixed columns referenced there in `lookup_fixed_columns`

This keeps the sparse path alive while restoring correctness:

- it avoids the broader fallback of materializing every fixed column for lookup
- and it restores agreement with the upstream `plonk_api` regression, which is
  the current highest-signal small reproducer for the remaining opening issue

## Permutation Chunking Regression

While continuing to chase the remaining `OpeningError`, the smaller
`tests/plonk_api.rs` regression exposed a semantic bug in the chunked
permutation-constraint path inside `evaluate_h(...)`.

The specific issue was:

- the chunked permutation path recomputed the running `delta` factor from the
  start of the row for every permutation set
- upstream PLONK semantics require that `delta` continue across all columns of
  the row, including columns that live in later permutation sets

That means the correct per-row factor for set `i` is not just:

- `beta * g_coset * omega^row`

but rather:

- `beta * g_coset * omega^row * DELTA^(i * chunk_len)`

Resetting it per set changes the right-hand side of the permutation product
constraint and produces a proof that is structurally valid but fails verifier
opening checks later with `OpeningError`.

The local patch now seeds each set with the correct `DELTA^(set_idx *
chunk_len)` offset before applying the per-column progression.

## Fixed Bug Inventory

Confirmed bugs fixed during the direct-lane debugging work:

1. Split transcript continuation lost Fiat-Shamir state

- Symptom:
  `Transcript error: Invalid ... point encoding in proof`
- Root cause:
  split finalization restored only proof bytes, not transcript state, and
  resumed appending from offset `0`
- Fix:
  `ReplayableCircuitTranscript` plus persisted replay-log snapshot and
  append-at-end restoration
- Regression coverage:
  - transcript snapshot/restore test
  - split base-proof roundtrip test in `plonk::prover`

2. `execute-wrapper-direct-verify` rejected `prove-finalize` JSON wrappers

- Symptom:
  `missing field proof_system` when passing the JSON emitted by
  `execute-wrapper-direct-prove-finalize`
- Root cause:
  the CLI verify path expected a bare `ProducedOuterProofArtifactBundle` but
  `prove-finalize` writes a larger execution-result wrapper containing
  `produced_bundle`
- Fix:
  CLI verify now accepts either the bare bundle or the wrapped
  `produced_bundle`
- Regression coverage:
  covered by CLI/workspace tests and real-command validation

3. Old persisted traces failed with opaque EOF errors

- Symptom:
  `failed to fill whole buffer` while reading a trace produced before transcript
  snapshot support
- Root cause:
  trace format grew a transcript-state section but the reader error did not
  explain the incompatibility
- Fix:
  explicit invalid-data diagnostics instructing the operator to rerun
  `prove-trace` with the current binary
- Regression coverage:
  exercised through direct command handling and trace reader checks

4. Chunked permutation constraints reset the `DELTA` progression per set

- Symptom:
  proofs stayed structurally valid but later failed verifier opening checks
  with `OpeningError`
- Root cause:
  the right-hand permutation product term restarted the `DELTA` factor at each
  permutation set instead of continuing across all columns in the row
- Fix:
  seed each set with `DELTA^(set_idx * chunk_len)` before continuing the
  per-column progression
- Regression coverage:
  upstream `plonk_api` reproducer

5. Sparse lookup fixed-column collection omitted columns needed by the full
   lookup argument

- Symptom:
  verifier-side `OpeningError` even after transcript parsing succeeded
- Root cause:
  the lookup sparse set derived only from `GraphEvaluator` missed some fixed
  columns still required by `lookup.input_expressions()` /
  `lookup.table_expressions()`
- Fix:
  augment `lookup_fixed_columns` with a direct recursive walk of those
  expressions
- Regression coverage:
  upstream `plonk_api` reproducer

This does not solve the hotspot by itself, but it is intended to avoid
rerunning the entire first stage every time we want to experiment on the
expensive second stage.

## Current Validation Status

The local patch should currently be treated as partially integrated but not yet
fully validated.

More specifically:

- the patch is active and the workspace compiles against it
- root and patch-local `cargo clippy` now pass with `-D warnings`
- the richer setup artifact is working
- the direct prove path that avoids rerunning `keygen_pk(...)` is working
- the split `execute-wrapper-direct-prove-trace` path is now working
- the remaining open problem is memory pressure in
  `execute-wrapper-direct-prove-finalize`

Latest validated split status:

- one successful `execute-wrapper-direct-prove-trace` run on
  `circom_multiplier2` produced:
  - `trace_elapsed_ms = 2228010`
  - older trace format size around `3.6 GiB`
- later experiments also changed the trace format while probing whether
  `h_poly` itself should move into the trace; those artifacts are obsolete and
  must be regenerated for the current format
- `execute-wrapper-direct-prove-finalize` still aborts with:
  `memory allocation of 268435456 bytes failed`

The patch now also emits finer-grained finalize checkpoints into the direct log
so the last completed subphase before one OOM abort is explicit.

That instrumentation now includes dedicated `finalize_for_h_poly()` markers:

- `midnight finalize: before compute_lagrange_polys`
- `midnight finalize: after compute_lagrange_polys`
- `midnight finalize: before sparse fixed cosets`
- `midnight finalize: after sparse fixed cosets`
- `midnight finalize: before permutation h key`
- `midnight finalize: after permutation h key`

Operational rule for experiments against this patch:

- if one code change affects setup artifact production, delete setup artifacts
  produced before that change
- if one code change affects persisted trace production or trace format, delete
  trace artifacts and trace logs produced before that change
- if one code change affects finalized proof-bundle production or finalize-side
  deserialization assumptions, delete finalized proof artifacts and finalize
  logs produced before that change

So the patch is still in an intermediate state:

- compile-time integrated
- runtime-successful for setup, direct prove, and split prove-trace
- runtime-not-yet-successful for split prove-finalize under current memory pressure
