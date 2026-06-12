# Midnight / Arkworks Stack Migration Plan

## Purpose

This document describes a dedicated migration plan for moving the repository to
the next dependency generation across the two main cryptography stacks it uses:

- the Midnight proving/circuit stack
- the arkworks reference / fixture / host-validation stack

Concretely, this plan targets a future branch that upgrades toward:

- `midnight-circuits 7.x`
- `midnight-proofs 0.8.x`
- `midnight-curves 0.3.x`
- `ark-bn254 0.6.x`
- `ark-ec 0.6.x`
- `ark-ff 0.6.x`
- `ark-groth16 0.6.x`
- `ark-relations 0.6.x`
- `ark-std 0.6.x`

This is intentionally a separate migration track rather than an incidental
cleanup inside ordinary feature work.

## Why This Needs Its Own Plan

The repository is not a simple application that happens to depend on a proving
library.

It currently relies on:

- a local `[patch.crates-io]` override for `midnight-proofs`
- direct use of Midnight chip/config internals
- cross-crate type coupling through `ff`, `PrimeField`, `Field`,
  `FromUniformBytes`, and related trait surfaces
- a reference path built on arkworks for fixtures, host validation, and narrow
  equivalence checks
- a direct outer proving lane whose artifact contracts and setup/prove/finalize
  lifecycle are already operational and measured

So this is not just “bump versions and fix compiler errors”.

It is a coordinated cryptography-stack migration with proof-generation,
serialization, circuit-configuration, and test-baseline consequences.

## Non-Goals

This plan is not intended to:

- change outer-statement semantics
- redesign the public wrapper package format
- replace the current Semaphore runbook
- remove the BN254 compatibility outer lane
- add new proving backends
- perform broad code cleanup unrelated to the dependency migration

If those changes become desirable, they should be staged separately once the
dependency migration is stable.

## Current Baseline

At the time this plan is written, the meaningful current baseline is:

- `midnight-circuits 6.1.x` in practice
- `midnight-curves 0.2.x`
- `midnight-proofs 0.7.x` with a local repo patch
- `ff 0.13.x`
- arkworks `0.5.x`

Important current constraints:

- the repo has already observed real breakage when `ff 0.14` enters the graph
  beside the Midnight/halo2 stack on `ff 0.13`
- the current direct lane depends on local `midnight-proofs` patch points such
  as richer setup/prove artifacts and split trace/finalize flow
- performance baselines, especially on `prove-finalize`, are already tracked
  and must remain comparable after migration

## Migration Philosophy

This migration should be handled as a branch-local compatibility project with
strict gates between phases.

Recommended principles:

- migrate the Midnight family together, not piecemeal
- migrate the arkworks family together, not piecemeal
- keep `ff` unified across the resolved graph at every stable checkpoint
- preserve the current direct-lane artifact/runbook contracts unless a change
  is explicitly accepted and documented
- keep the BLS12-381 outer lane as the primary acceptance lane
- treat the BN254-hosted outer lane as a compatibility lane that must still
  compile and pass its focused coverage

## High-Level Risk Areas

### 1. Trait-surface breakage

Likely breakpoints include:

- `ff::Field`
- `ff::PrimeField`
- `FromUniformBytes`
- field constants like `ZERO` / `ONE`
- encoding helpers like `to_repr`, `from_repr`, `from_str_vartime`

This risk is especially high where the repo mixes:

- direct `ff` imports
- `midnight-curves` field types
- `halo2curves`-adjacent trait bounds

### 2. Midnight chip/config breakage

Likely breakpoints include:

- chip constructors
- config-shell wiring
- `ComposableChip` usage
- `FromScratch` / testing-only surfaces
- public-input exposure helpers
- Poseidon / native / foreign-field chip configuration

### 3. Local patch drift

The repo carries a local patch for `midnight-proofs`.

That means a `0.8.x` migration is blocked until one of these is true:

- the patch can be rebased cleanly onto the new upstream line
- the needed functionality exists upstream and the patch can shrink or vanish
- the repo accepts a different direct-lane artifact/proving flow

This is the single strongest reason the Midnight migration should be staged
before any broad arkworks cleanup.

### 4. Artifact / serialization drift

Potentially affected surfaces:

- proving-key persistence
- verifier-param serialization
- proof bytes
- verification-key bytes
- setup/prove/trace/finalize artifact bundle shapes

Even if the compiler changes are small, this area can silently break operator
runbooks.

### 5. Performance regressions

A successful compile is not enough.

The migration must also be checked against:

- setup wall-clock
- prove-trace wall-clock
- prove-finalize wall-clock
- `circuit_k`
- public-input count
- peak memory or obvious memory-shape regressions where measurable

## Recommended Execution Order

### Phase 0. Freeze the acceptance baseline

Goal:

- record exactly what “still works” before the migration branch begins

Required outputs:

- one documented dependency baseline
- one focused test baseline
- one direct-lane runtime baseline for the official BLS12-381 lane
- one note describing the local `midnight-proofs` patch surface that must be
  preserved or consciously replaced

Minimum verification:

- `cargo test -q -p wrapper-circuits outer::tests --lib`
- `cargo test -q -p wrapper-backends outer::tests --lib`
- `cargo test -q -p wrapper-tests --lib`
- the official Semaphore direct-lane commands from `README.md`

Exit condition:

- migration branch starts from a known-good baseline with comparable metrics

### Phase 1. Audit and classify the local `midnight-proofs` patch

Goal:

- determine exactly which patch hunks are mandatory for repo function today

Tasks:

1. list every public symbol added or behavior changed in
   `patches/midnight-proofs`
2. classify each one as:
   - mandatory now
   - nice-to-have
   - obsolete if upstream `0.8.x` already covers it
3. map each patch item to the repo call sites that depend on it

Primary files:

- `patches/midnight-proofs/`
- `crates/wrapper-backends/src/outer/direct/proving.rs`
- `docs/decisions/0004-local-midnight-proofs-patch.md`

Exit condition:

- there is a concrete “patch carry-forward” list, not just a vague expectation

### Phase 2. Upgrade the Midnight family only

Goal:

- move to the target Midnight line while keeping arkworks pinned on `0.5.x`

Recommended target first:

- `midnight-circuits 7.x`
- `midnight-curves 0.3.x`
- `midnight-proofs 0.8.x`

Do not simultaneously upgrade arkworks here.

Tasks:

1. update workspace dependency declarations
2. port the local `midnight-proofs` patch or retire replaced pieces
3. fix all Midnight/`ff`/chip/config/compiler breakage
4. restore the direct outer setup/prove/verify lane
5. rerun focused runtime baselines on the BLS12-381 lane

Primary acceptance lane:

- `midnight-bls12-381-host`

Secondary acceptance lane:

- `midnight-bn254-host`

Exit condition:

- both outer lanes compile
- focused tests pass
- official BLS12 Semaphore flow works again
- artifact serialization is still accepted by repo tooling

### Phase 3. Upgrade the arkworks family only

Goal:

- move the host/reference stack to `0.6.x` after the Midnight migration is
  already stable

Tasks:

1. upgrade the full arkworks family together
2. fix host verification / fixture / typed conversion breakage
3. revalidate that host-side reference checks still agree with circuit-side
   behavior

Primary files:

- `crates/wrapper-circuits/src/groth16/reference.rs`
- `crates/wrapper-circuits/src/test_support.rs`
- `crates/wrapper-circuits/src/bn254/tests/`
- `crates/wrapper-backends/src/snarkjs.rs`

Exit condition:

- all arkworks-backed reference checks pass on the migrated Midnight line

### Phase 4. Reconcile docs, runbooks, and baselines

Goal:

- make the migration visible and durable for future work

Tasks:

1. update `README.md`
2. update `docs/architecture.md`
3. update `docs/semaphore-direct-execution-playbook.md`
4. update any direct-lane metrics/baseline docs that changed materially
5. document whether the local `midnight-proofs` patch survived, shrank, or was
   eliminated

Exit condition:

- docs describe the new dependency reality and operator commands truthfully

## Detailed Acceptance Criteria

The migration should not be called complete until all of the following are
true:

1. dependency graph:
   - the intended Midnight line is resolved in `Cargo.lock`
   - the intended arkworks line is resolved in `Cargo.lock`
   - `ff` is not split in a way that breaks repo field traits
2. compile:
   - `wrapper-circuits`, `wrapper-backends`, `wrapper-cli`, and
     `wrapper-tests` compile
3. focused correctness:
   - outer-circuit tests pass
   - backend adaptation tests pass
   - wrapper integration tests pass
4. direct lane:
   - setup works
   - prove-trace works
   - prove-finalize works
   - verify works
5. artifact compatibility:
   - produced setup/proof/VK artifacts still match repo tooling expectations
6. performance:
   - any significant runtime or memory regression is measured and documented

## Strong Reasons To Stage This Migration Separately

The following are the strongest arguments against doing this inline with normal
feature work:

- the repo depends on a patched proving backend, so upstream version changes
  are not purely declarative
- the `ff` trait surface is a real compatibility boundary, not an abstract
  nuisance
- the direct outer lane is already operational, so regressions are expensive
  and user-visible
- performance can regress even when the compiler is happy
- this migration touches infrastructure used by every future proving experiment

## Suggested Worktree / Branch Policy

Recommended branch posture:

- do this in a dedicated migration branch
- avoid mixing feature work into that branch
- checkpoint after each phase

Recommended commit grouping:

1. baseline documentation and patch audit
2. Midnight-family migration
3. direct-lane restoration
4. arkworks-family migration
5. documentation and metrics refresh

## Minimal First Step

If this plan is activated, the first concrete step should be:

1. create a migration branch
2. capture the current baseline and patch inventory
3. attempt only the Midnight-family upgrade first
4. do not touch arkworks until the direct BLS12 lane is green again

That is the smallest execution slice that still respects the real dependency
risk structure of this repository.
