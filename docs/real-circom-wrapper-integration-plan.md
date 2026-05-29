# Real Circom Wrapper Integration Plan

## Purpose

This document records the remaining path from a real `.circom`-origin Groth16
artifact set to a real outer-wrapper proof on the current direct lane.

It is intentionally scoped to the existing repository architecture rather than
to a hypothetical future verifier stack.

## Current Position

The repository already has the core pieces of a real `.circom` integration:

- real `snarkjs` Groth16 BN254 parsing
- generic artifact-bundle normalization
- `WrapperJob` and `WrapperExecutionPackage` planning
- the canonical `OuterWrapperCircuit`
- a real direct Halo2/Midnight setup/prove/verify lane
- committed `circom_multiplier2` and `semaphore` fixtures

This means the repo is already past the “can we ingest a real Circom proof
tuple at all?” stage.

## What Is Already Implemented

### Real artifact ingestion

Implemented:

- `proof.json`
- `public.json`
- `verification_key.json`

through:

- `crates/wrapper-backends/src/snarkjs.rs`
- `crates/wrapper-backends/src/groth16.rs`

### Real wrapper planning

Implemented:

- `Groth16Bn254ArtifactBundle`
- `WrapperJob`
- `WrapperExecutionPackage`

through:

- `crates/wrapper-backends/src/groth16.rs`
- `crates/wrapper-core/src/job.rs`
- `crates/wrapper-core/src/package.rs`

### Real outer execution lane

Implemented:

- direct setup
- direct prove
- direct verify
- split prove trace / prove finalize

through:

- `crates/wrapper-backends/src/outer/direct/`
- `crates/wrapper-cli/src/main.rs`

## Current Remaining Blockers

The remaining blockers are now mostly operational rather than architectural.

Primary blocker:

- memory pressure in `execute-wrapper-direct-prove-finalize`

Secondary blockers:

- finalize observability still needs to be more systematic
- artifact hygiene remains important when setup/trace/finalize formats change
- heavier fixtures remain expensive enough that reproducible developer
  experimentation requires discipline

## Recommended Completion Order

For the current phase, the shortest path to a reliably real `.circom` ->
outer-wrapper flow is:

1. keep using the canonical direct outer lane
2. keep the richer setup artifact and local `midnight-proofs` patch
3. improve finalize observability and last-checkpoint diagnosis
4. stabilize finalize memory behavior
5. only after that, revisit speed-oriented `h_poly` follow-up work

This ordering matters because the current gap is not proof parsing or circuit
semantics.
It is finalize reliability.

## What Not To Do Next

For this path, the repository should not next:

- pivot the critical path to canonical R1CS
- widen the inner-proof parser surface beyond the current narrow Groth16 BN254 lane
- generalize backend APIs before the direct lane is operationally reliable

## Key Companion Documents

Use this document together with:

- `docs/outer-prover-strategy-plan.md`
- `docs/decisions/0003-direct-outer-setup-cost-reduction.md`
- `docs/decisions/0004-local-midnight-proofs-patch.md`
- `docs/plans/0006-finalize-checkpoint-profiling-plan.md`
- `docs/r1cs-backend-status.md`

## Short Summary

The real `.circom` -> outer-wrapper path is no longer blocked by missing
artifact ingestion or missing direct backend plumbing.

It is now blocked primarily by prover-finalize memory behavior on the current
direct lane.
