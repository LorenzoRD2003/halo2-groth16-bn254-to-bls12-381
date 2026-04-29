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

Current state of the patch:

- it is active through `[patch.crates-io]`
- it compiles in the workspace
- the direct outer backend now uses:
  - `keygen_pk_base(...)` during setup
  - `BaseProvingKey::read(...)` during prove
  - `create_proof_from_base(...)` during prove
  - `create_proof_trace_from_base(...)` for the first stage of prove
  - `finalise_proof_from_base_trace(...)` for the second stage of prove

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

The current most suspicious remaining hotspot is eager extended-domain coset
materialization inside `compute_h_poly(...)` in the patched prover:

- `advice_cosets`
- `instance_cosets`

Those allocations are the next target for memory reduction work.

The patch also now supports a practical experimentation split:

- compute and persist the first-stage prover trace before `compute_h_poly(...)`
- finalize from that persisted trace later

This does not solve the hotspot by itself, but it is intended to avoid
rerunning the entire first stage every time we want to experiment on the
expensive second stage.

## Current Validation Status

The local patch should currently be treated as partially integrated but not yet
fully validated.

More specifically:

- the patch is active and the workspace compiles against it
- the richer setup artifact is working
- the direct prove path that avoids rerunning `keygen_pk(...)` is working
- but the split `execute-wrapper-direct-prove-trace` path is still considered
  broken

Latest valid observed split failure:

- `execute-wrapper-direct-prove-trace` fails with:
  `midnight create_proof_trace_from_base failed: The constraint system is not satisfied`
- the last reliable backend log ends at:
  - `prove-trace: entering create_proof_trace_from_base`

So the patch is still in an intermediate state:

- compile-time integrated
- runtime-successful for setup and the non-split direct prove path experiments
- runtime-not-yet-correct for the split trace path
