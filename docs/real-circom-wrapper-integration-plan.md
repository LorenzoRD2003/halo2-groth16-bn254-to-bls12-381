# Real Circom Wrapper Integration Plan

## Purpose

This document describes the implementation path to finish the current wrapper
lane for a real `.circom`-origin proof bundle and carry it through a real outer
proof flow.

The project originally explored an outer `groth16-bls12-381` target. The code
and documentation work completed so far now make one thing explicit:

- the canonical Halo2/Midnight outer circuit already contains the BN254
  non-native verifier logic we need
- the canonical R1CS line is real and useful, but still incomplete for the
  full non-native pairing core
- therefore the shortest path to a real end-to-end wrapper flow is the
  canonical Halo2/Midnight outer circuit plus a real direct prover / serializer
  backend for that circuit

This plan is therefore re-centered around:

```text
snarkjs BN254 artifacts
  -> canonical OuterWrapperCircuit (Halo2/Midnight)
  -> real Halo2/Midnight proof / VK / public artifacts
```

The canonical R1CS line remains valuable, but it is now explicitly treated as
an alternate / future backend lane rather than the delivery critical path.

## Goal

Given a real `.circom`-origin artifact set:

- `proof.json`
- `public.json`
- `verification_key.json`

produce and verify a real outer artifact set for the canonical outer
Halo2/Midnight circuit:

- `wrapper-proof.json`
- `wrapper-public.json`
- `wrapper-verification-key.json`

The proof-system family of those artifacts should match the real backend chosen
for the canonical outer circuit. If that backend is Halo2/Midnight PLONKish
over BLS12-381, then the produced artifacts should describe that proof system
honestly rather than pretending to be Groth16.

## Current State

What already exists:

- generic `snarkjs` Groth16 BN254 proof / VK / public-input parsing
- generic `ArtifactSetLoader` for complete BN254 artifact bundles
- `Groth16Bn254ArtifactBundle -> WrapperJob -> WrapperExecutionPackage ->
  WrapperExecutionResult`
- a real canonical `OuterWrapperCircuit`
- strict outer statement contract validation
- backend-side adaptation from package + artifacts into canonical outer circuit
  input
- backend-side planning surfaces for:
  - package-oriented artifact planning
  - direct canonical outer-circuit planning
- expected/planned vs produced output modeling in `wrapper-core`
- canonical R1CS infrastructure and Arkworks adapter as an alternate/future lane

What does not exist yet:

- a real direct prover / serializer for `OuterWrapperCircuit`
- a produced outer proof payload
- a produced outer verification key payload
- backend-level verification for that produced outer proof
- a CLI command that runs the real outer path

## Architectural Stance

There are now three distinct layers:

1. Inner artifact ingestion
   - `snarkjs` BN254 parsing and package construction

2. Canonical outer circuit
   - `OuterWrapperCircuit` in Halo2/Midnight
   - this is the delivery critical path

3. Canonical R1CS line
   - deterministic identity and alternate backend path
   - not the critical path for the first real wrapper flow

This plan focuses on layer 2.

## Implementation Plan

### 1. Freeze the Output Contract for the Direct Outer-Circuit Lane

Deliverable:

- one honest output contract for the real direct outer-circuit backend

Tasks:

- stop assuming the final artifacts must remain Groth16-shaped if the chosen
  real backend is Halo2/Midnight PLONKish
- define the produced artifact contract honestly:
  - protocol label
  - curve label
  - proof payload shape
  - verification-key payload shape
  - public-input payload shape
- keep the ordered public-input contract unchanged

Primary location:

- `wrapper-core/src/output.rs`
- `wrapper-backends/src/outer.rs`

### 2. Finalize the Direct Canonical Outer-Circuit Backend Surface

Deliverable:

- one internal backend surface for setup / prove / verify over
  `OuterWrapperCircuit`

Current status:

- planning surfaces now exist:
  - `CanonicalOuterCircuitProofBackend`
  - `plan_direct_outer_circuit_setup(...)`
  - `plan_direct_outer_circuit_proof(...)`

Remaining tasks:

- add the concrete setup / prove / verify methods or hook points for the real
  backend
- keep the package-oriented backend trait as the public orchestrator
- keep the low-level proving surface centered on `OuterWrapperCircuit`

Primary location:

- `wrapper-backends/src/outer.rs`

### 3. Choose the Real Direct Proving Stack

Deliverable:

- one concrete proving stack for the canonical outer Halo2/Midnight circuit

Decision target:

- prefer the direct Halo2/Midnight path over R1CS lowering for the first real
  wrapper flow

Tasks:

- decide which concrete prover / serializer API to use
- document:
  - setup assumptions
  - serialization format
  - verification flow
  - compatibility constraints
- keep all stack-specific types behind the backend implementation

Primary location:

- `wrapper-backends/src/outer.rs`
- a dedicated backend helper module if needed

This is the main blocker today.

### 4. Implement Real Setup for the Canonical Outer Circuit

Deliverable:

- a produced verification key artifact for the outer circuit

Tasks:

- run real setup over `OuterWrapperCircuit`
- serialize the resulting verification key into the chosen output shape
- validate:
  - public-input arity
  - stable circuit build status
  - protocol / curve labels

Primary location:

- `wrapper-backends`
- `wrapper-core/src/output.rs`

### 5. Implement Real Proving for the Canonical Outer Circuit

Deliverable:

- a produced proof artifact for the outer circuit

Tasks:

- run real proving over `OuterWrapperCircuit`
- serialize the resulting proof
- export public inputs in wrapper statement order
- keep failure cases explicit:
  - malformed inner bundle
  - invalid outer input adaptation
  - synthesis failure
  - serialization failure

Primary location:

- `wrapper-backends`

### 6. Implement Real Verification for the Produced Outer Artifacts

Deliverable:

- backend-level verification for the produced outer proof

Tasks:

- verify against the produced VK and ordered public inputs
- make verification part of the acceptance path
- return a short verdict suitable for CLI and integration tests

Primary location:

- `wrapper-backends`
- `wrapper-tests`

### 7. Promote the Semaphore Lane to a Real End-to-End Integration Test

Deliverable:

- one end-to-end test over a real `.circom`-origin bundle

Tasks:

- load the inner BN254 bundle through the generic loader
- build the wrapper package
- adapt into `OuterWrapperCircuit`
- run real setup / prove / verify
- validate produced artifacts

Recommended fixture:

- Semaphore

Recommended future contrast fixture:

- a smaller non-ECC `.circom` circuit

### 8. Add a CLI Command for the Real Path

Deliverable:

- one CLI command that runs the real direct outer-circuit lane

Suggested command shape:

```text
wrapper-cli prove-outer \
  --proof <proof.json> \
  --public <public.json> \
  --vk <verification_key.json> \
  [--public-input-name ...] \
  --output-dir <dir>
```

Tasks:

- load inner bundle
- build wrapper package
- run real setup / prove / verify as requested
- write:
  - `wrapper-proof.json`
  - `wrapper-public.json`
  - `wrapper-verification-key.json`
- print a short summary of:
  - selected backend
  - outer circuit build status
  - verification result

## Validation Matrix

Minimum validation before claiming end-to-end completion:

- unit tests for:
  - output serialization
  - direct backend setup / prove / verify adapters
  - outer statement contract checks
- integration tests for:
  - canonical fixture smoke path
  - real `.circom` fixture such as Semaphore
  - produced-proof verification
- CLI smoke test for:
  - full outer-proof generation
  - proof verification

## Recommended Execution Order

1. Freeze the direct-output contract.
2. Finalize the direct outer-circuit backend surface.
3. Choose the real proving stack.
4. Implement setup.
5. Implement prove.
6. Implement verify.
7. Promote Semaphore to a full integration test.
8. Add the CLI command.

## Explicit Non-Goals

- pretending the first real backend still has to be Groth16 if the actual
  chosen backend is PLONKish / Halo2
- treating canonical R1CS as the critical path for the first delivery
- reimplementing proof-system internals manually
- broadening into a general-purpose proof ecosystem before the direct wrapper
  path is real
