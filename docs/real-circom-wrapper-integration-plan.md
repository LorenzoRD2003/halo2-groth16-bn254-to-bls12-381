# Real Circom Wrapper Integration Plan

## Purpose

This document describes the implementation path to finish the current wrapper
lane for a real `.circom`-origin Groth16 BN254 proof and carry it through the
planned outer `groth16-bls12-381` flow.

The plan is intentionally generic over application circuits. The current
Semaphore fixture is the validation case because it exercises a non-trivial
ECC-heavy circuit, but the interfaces below should remain usable for any
`snarkjs`-shaped Groth16 BN254 artifact set.

## Current State

What already exists:

- generic `snarkjs` Groth16 BN254 proof / VK / public-input parsing
- generic `ArtifactSetLoader` for complete BN254 artifact bundles
- a `Groth16Bn254ArtifactBundle -> WrapperJob -> WrapperExecutionPackage ->
WrapperExecutionResult` planning lane
- explicit expected output shapes for outer `groth16-bls12-381` artifacts
- a placeholder `OuterGroth16Backend` that materializes a partial outer bundle
- a real Semaphore fixture that validates the full planning/stub path

What does not exist yet:

- a real outer Groth16 BLS12-381 circuit / proving backend
- a real outer proof payload
- a real outer verification key payload derived from circuit setup
- end-to-end proof generation and verification for the outer proof

## Goal

Given a real `.circom`-origin `snarkjs` artifact set:

- `proof.json`
- `public.json`
- `verification_key.json`

produce and verify a real outer `groth16-bls12-381` artifact set:

- `wrapper-proof.json`
- `wrapper-public.json`
- `wrapper-verification-key.json`

while preserving the current generic boundaries:

- `wrapper-backends` stays generic over artifact ingestion and outer backend
  contracts
- `wrapper-core` stays domain-oriented
- application-specific public-input naming stays in fixture/domain layers, not
  generic parsers

## Implementation Plan

### 1. Freeze the Outer Statement Contract

Deliverable:

- one canonical rule for how outer public inputs are derived from the inner
  proof statement

Tasks:

- decide whether the outer statement mirrors inner verifier public inputs
  exactly, or whether it exposes an application-shaped statement and proves the
  mapping
- keep this decision generic: the wrapper core should only need ordered named
  public inputs, not Semaphore-specific semantics
- add tests that reject mismatches between:
  - expected outer statement arity
  - inner verifier public-input arity
  - verification-key IC length

Suggested location:

- `wrapper-core`

### 2. Split the Outer Output Model Into “Planned” vs “Produced”

Deliverable:

- a type distinction between:
  - expected/planned outer artifact shapes
  - actually produced outer artifacts

Tasks:

- keep the current `ExpectedWrapperArtifacts` as the planning contract
- introduce a produced-artifact type that represents a real generated outer
  proof bundle, not a placeholder
- make the produced type strict enough that `proof` and `verification_key`
  cannot stay absent once the real backend is wired

Suggested location:

- `wrapper-core/src/output.rs`

### 3. Introduce a Real Outer Backend Interface

Deliverable:

- an `OuterGroth16Backend` contract that separates:
  - planning/materialization
  - actual proving/setup/verification

Tasks:

- extend or replace the current placeholder backend with methods such as:
  - `prepare(...)`
  - `setup(...)`
  - `prove(...)`
  - `verify(...)`
- keep the input generic:
  - `WrapperExecutionPackage`
- keep the output generic:
  - outer Groth16 artifact bundle
- avoid leaking application-specific semantics into the backend trait

Suggested location:

- `wrapper-backends/src/outer.rs`

### 4. Choose and Encapsulate the Real Groth16 BLS12-381 Stack

Deliverable:

- one concrete outer backend implementation behind the generic trait

Tasks:

- choose the proving stack for outer `groth16-bls12-381`
- encapsulate all stack-specific details behind the backend implementation
- avoid spreading library-specific types across `wrapper-core`
- document:
  - setup assumptions
  - serialization conventions
  - proof/VK/public-input artifact compatibility

Suggested location:

- `wrapper-backends/src/outer/` or a dedicated backend module subtree

### 5. Build the Outer Circuit Input Adapter

Deliverable:

- an adapter that turns a `WrapperExecutionPackage` into the real witness/input
  format required by the chosen outer backend

Tasks:

- map:
  - inner proof payload
  - inner verification key payload
  - ordered public inputs
  - outer statement
    into the exact witness/config input required by the outer circuit
- make failure cases explicit:
  - missing proof payload
  - malformed VK
  - arity mismatch
  - unsupported statement layout

Suggested location:

- `wrapper-backends`

### 6. Replace the Placeholder VK Skeleton With Real Setup Output

Deliverable:

- a real `wrapper-verification-key.json`

Tasks:

- run the outer setup flow for the selected backend
- serialize the resulting VK into the agreed output shape
- verify that the emitted shape matches the current expected model:
  - `protocol`
  - `curve`
  - `nPublic`
  - `IC`
  - point encoding conventions

Suggested tests:

- compare actual emitted keys/fields against the planned shape contract

### 7. Replace `proof: null` With a Real Outer Proof

Deliverable:

- a real `wrapper-proof.json`

Tasks:

- run the outer prover on a real `.circom`-origin inner bundle
- serialize the proof using the agreed shape
- verify that the output payload matches:
  - key names
  - point encoding
  - protocol/curve labels

Suggested tests:

- proof round-trip/load/verify through the backend

### 8. Add Real Outer Verification

Deliverable:

- one path that verifies the produced outer proof against the produced outer VK
  and outer public inputs

Tasks:

- add backend-level verification for the produced outer bundle
- use the same ordered public inputs as the exported `wrapper-public.json`
- make this part of the integration acceptance lane

Suggested location:

- `wrapper-backends`
- `wrapper-tests`

### 9. Promote the Real Circom Fixture Lane to a First-Class Integration Test

Deliverable:

- one reusable end-to-end integration test over a real `.circom`-origin proof

Tasks:

- keep the fixture generic:
  - load artifacts via the generic `ArtifactSetLoader`
- allow the fixture layer to supply semantic public-input names
- validate the full chain:
  - load BN254 bundle
  - build wrapper package
  - produce outer bundle
  - verify outer bundle

Current recommended real fixture:

- Semaphore

Additional recommended future fixture:

- a simpler non-ECC `.circom` circuit for contrast

### 10. Add a CLI Command for the Real Path

Deliverable:

- one CLI command that exercises the real outer path

Suggested command shape:

```text
wrapper-cli prove-outer-groth16 \
  --proof <proof.json> \
  --public <public.json> \
  --vk <verification_key.json> \
  [--public-input-name ...] \
  --output-dir <dir>
```

Tasks:

- load inner bundle
- build wrapper package
- run the real outer backend
- write:
  - `wrapper-proof.json`
  - `wrapper-public.json`
  - `wrapper-verification-key.json`
- optionally run verification and print a short verdict

## Validation Matrix

Minimum validation before claiming end-to-end completion:

- unit tests for:
  - output-model serialization
  - backend setup/prove/verify adapters
  - statement/public-input arity checks
- integration tests for:
  - canonical small fixture
  - real `.circom` fixture such as Semaphore
- CLI smoke test for:
  - full outer-proof generation
  - outer-proof verification

## Recommended Execution Order

1. Freeze the outer statement contract.
2. Introduce produced-artifact types distinct from planned types.
3. Finalize the real outer backend trait and choose the stack.
4. Implement setup and real VK emission.
5. Implement prover and real proof emission.
6. Implement verification.
7. Promote the real `.circom` fixture lane to a full integration test.
8. Add the CLI command for real outer generation.

## Non-Goals For This Plan

- moving application-specific naming into generic backend parsing
- pretending the current placeholder output is already a real proof
- broadening the current repo into a general-purpose proof ecosystem before the
  real end-to-end wrapper path exists
