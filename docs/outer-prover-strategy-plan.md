# Outer Prover Strategy Plan

## Purpose

This document records the current prover strategy for the canonical
`OuterWrapperCircuit`.

It answers three practical questions:

1. which outer circuit is canonical today
2. which backend lane is the current delivery-critical path
3. what the remaining prover-side blocker is

This is not an ADR and it is not reopening the old question of whether the
repository should define multiple competing outer circuits.

## Current Decision

The canonical outer circuit remains:

- `crates/wrapper-circuits/src/outer/mod.rs`
- semantic type: `OuterWrapperCircuit`

The current concrete prover lane remains:

- direct Halo2/Midnight backend
- `wrapper-backends/src/outer/direct/`
- public backend surface:
  - `MidnightDirectOuterBackendBn254Host`
  - `MidnightDirectOuterBackendBls12Host`

Current lane policy:

- `MidnightDirectOuterBackendBls12Host` is the official outer lane
- `MidnightDirectOuterBackendBn254Host` remains a compatibility/testing lane

The canonical circuit question is therefore considered settled for the current
phase:

- `wrapper-circuits` owns the outer circuit semantics
- `wrapper-backends` owns setup/prove/verify materialization for that circuit

## What The Repository Can Do Today

The direct outer lane now supports:

- setup
- prove
- verify
- split prove trace / prove finalize

Operationally, the lane to prefer for production-facing work is:

- `MidnightDirectOuterBackendBls12Host`

The BN254-hosted lane remains useful for:

- comparative regressions
- compatibility checks
- historical baselines already captured in the repository

In current CLI terms, the practical direct-lane commands are:

- `execute-wrapper-direct-setup`
- `execute-wrapper-direct-prove`
- `execute-wrapper-direct-prove-trace`
- `execute-wrapper-direct-prove-finalize`
- `execute-wrapper-direct-verify`

The direct path is already far beyond a stub:

- it adapts real Groth16 BN254 artifact bundles into the canonical outer input
- it constructs the real hosted outer circuit
- it emits honest setup/proof/VK artifact shapes
- it validates proof and verification-key serialization against the active
  backend contract

## What The Strategy Is Not

The current strategy is not:

- “switch to the canonical R1CS lane first”
- “define a second competing outer circuit”
- “generalize the backend surface before the direct lane is reliable”

The canonical R1CS line is strategically important, but it is still an
alternate / future backend lane rather than the delivery-critical path for the
first real `.circom` -> outer-wrapper flow.

## Current Critical Blocker

The main remaining blocker is no longer missing setup/prove plumbing.

The current blocker is:

- memory pressure in `execute-wrapper-direct-prove-finalize`

Current observed state:

- `execute-wrapper-direct-prove-trace` succeeds
- `execute-wrapper-direct-prove-finalize` still aborts under current memory
  pressure
- the most suspicious site remains eager prover-side state around
  `compute_h_poly(...)`

Relevant currently accepted references:

- `docs/decisions/0003-direct-outer-setup-cost-reduction.md`
- `docs/decisions/0004-local-midnight-proofs-patch.md`
- `docs/plans/0006-finalize-checkpoint-profiling-plan.md`
- `docs/h-poly-followup-speed-plan.md`

## Current Strategy

The retained strategy for the direct outer prover lane is:

1. keep the canonical outer circuit unchanged as the single circuit source of truth
2. keep the local `midnight-proofs` patch that enables the richer setup/prove split
3. keep the split `prove-trace` / `prove-finalize` workflow
4. improve observability first, especially on the finalize path
5. solve finalize memory reliability before pursuing speed-oriented follow-up work

In practice this means:

- do not spend the next cycle widening APIs
- do not pivot the delivery path to canonical R1CS
- do not optimize `h_poly` throughput before finalize is memory-stable

## Recommended Reading Order

If the task is specifically about the current outer prover path, read in this order:

1. `docs/decisions/0003-direct-outer-setup-cost-reduction.md`
2. `docs/decisions/0004-local-midnight-proofs-patch.md`
3. `docs/plans/0006-finalize-checkpoint-profiling-plan.md`
4. `crates/wrapper-backends/src/outer/direct/proving.rs`
5. `patches/midnight-proofs/src/plonk/prover.rs`
6. `docs/h-poly-followup-speed-plan.md`

## Short Summary

Today the repository does not need a new outer prover strategy in the sense of
choosing a different backend family.

It already has the strategy:

- canonical Halo2/Midnight outer circuit
- direct setup/prove/verify lanes, with `BLS12-381` as the official one
- richer setup artifact plus split prove stages
- finalize memory reduction as the remaining practical blocker
