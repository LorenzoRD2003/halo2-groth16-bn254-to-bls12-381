# Real Circom Wrapper Integration Plan

## Purpose

This document describes the implementation path to finish the current wrapper
lane for a real `.circom`-origin Groth16 BN254 proof and carry it through the
planned outer flow.

This plan is now grounded in the repo's landed canonical R1CS work:

- canonical R1CS model and normalization
- deterministic Halo2-style cell/equality lowering
- explicit metadata boundary for future Halo2/Midnight extraction
- canonical circuit identity via `R1csIdentityHash`
- zkInterface-style internal export bridge
- first Arkworks adapter from canonical `R1csCircuit` to Groth16-compatible
  `ConstraintSynthesizer`

As a result, the remaining work is no longer "invent the lowering story". The
remaining work is "close the real backend and artifact-production path".

Important project stance:

- the canonical R1CS line is real and strategically important
- but it should currently be treated as an **alternate backend / later phase**
- the **delivery critical path** for the first real `.circom` wrapper flow
  remains the canonical Halo2/Midnight outer circuit plus a real prover /
  serializer backend for that circuit

The plan remains intentionally generic over application circuits. The current
Semaphore fixture is the validation case because it exercises a non-trivial
ECC-heavy circuit, but the interfaces below should remain usable for any
`snarkjs`-shaped Groth16 BN254 artifact set.

## Current State

What already exists:

- generic `snarkjs` Groth16 BN254 proof / VK / public-input parsing
- generic `ArtifactSetLoader` for complete BN254 artifact bundles
- a `Groth16Bn254ArtifactBundle -> WrapperJob -> WrapperExecutionPackage ->
WrapperExecutionResult` planning lane
- explicit expected output shapes for outer wrapper artifacts
- `OuterGroth16Backend` with explicit `prepare / setup / prove / verify`
  boundaries
- a real Semaphore fixture that validates the planning/stub path
- a real Halo2/Midnight outer wrapper circuit as the canonical outer circuit
  source of truth
- canonical R1CS lowering infrastructure under
  `crates/wrapper-circuits/src/r1cs/`
- canonical circuit identity via `R1csCircuit::identity_hash()`
- a first Arkworks Groth16 adapter from canonical `R1csCircuit` into
  `ark_relations::r1cs::ConstraintSynthesizer`
- end-to-end Groth16 setup/prove/verify on a small canonical R1CS smoke circuit
- documented positioning of the canonical R1CS line as an alternate / future
  backend lane

What still does not exist:

- a real prover/serializer for the canonical outer Halo2/Midnight circuit
- a real produced outer proof payload
- a real produced outer verification key payload derived from setup
- end-to-end proof generation and verification for the real outer wrapper lane
- a CLI command that exercises the real outer path

## Goal

Given a real `.circom`-origin `snarkjs` artifact set:

- `proof.json`
- `public.json`
- `verification_key.json`

produce and verify a real outer artifact set:

- `wrapper-proof.json`
- `wrapper-public.json`
- `wrapper-verification-key.json`

while preserving the current generic boundaries:

- `wrapper-backends` stays generic over artifact ingestion and outer backend
  contracts
- `wrapper-core` stays domain-oriented
- `wrapper-circuits` owns the canonical outer circuit and canonical R1CS
  lowering
- application-specific public-input naming stays in fixture/domain layers, not
  generic parsers

## Architectural Reality

The repo now has two distinct proving-related layers:

1. A canonical R1CS layer.
   This is deterministic, auditable, hashable, and already has an Arkworks
   Groth16 adapter.

2. A canonical outer wrapper circuit layer.
   This lives in Halo2/Midnight and remains the single source of truth for the
   outer circuit itself.

The current real integration problem is therefore:

> connect real `.circom`-origin inner artifacts to the canonical outer circuit,
> then materialize real produced outer artifacts through one concrete backend.

The canonical R1CS work materially reduces risk around circuit identity and
future CRS binding, but it does not by itself finish the real outer wrapper
flow.

For the first real wrapper delivery, the practical priority is:

- keep the outer Halo2/Midnight circuit canonical
- add a real prover / serializer backend for that circuit
- use the canonical R1CS line as an alternate / future backend path, not as the
  immediate blocker

## Rebased Implementation Plan

### 1. Keep the Outer Statement Contract Frozen and Explicit

Deliverable:

- one canonical rule for how outer public inputs are derived from the inner
  proof statement

Current status:

- implemented and regression-tested already

Completed scope:

- the current "ordered named public inputs" contract is explicit
- application-specific semantics remain outside generic parsers
- the package/backend path rejects mismatches between:
  - expected outer statement arity
  - inner verifier public-input arity
  - verification-key IC length

Remaining work:

- keep this frozen while finishing the real backend path

Primary location:

- `wrapper-core`
- `wrapper-circuits/src/outer/`
- `wrapper-backends/src/outer.rs`

### 2. Treat Canonical R1CS as the Long-Term CRS-Binding Source of Truth

Deliverable:

- no further ambiguity about circuit identity

Current status:

- implemented for the canonical R1CS layer, and explicitly represented in the
  planning/output model as an attachable identity record

Tasks:

- keep `R1csCircuit::identity_hash()` as the canonical CRS-binding identity for
  the lowering path
- do not let backend serialization formats become identity sources
- keep setup/proving metadata able to carry canonical circuit identity when the
  outer circuit has a canonical R1CS lowering

Remaining work:

- attach a real canonical circuit identity to produced outer artifacts once the
  canonical outer circuit itself is lowered to canonical R1CS

Primary location:

- `crates/wrapper-circuits/src/r1cs/`

### 3. Keep the Canonical R1CS Backend as an Alternate / Future Lane

Deliverable:

- one documented alternate backend lane rooted in canonical `R1csCircuit`

Current status:

- canonical `R1csCircuit` exists
- Arkworks Groth16 adapter exists
- the R1CS backend is explicitly treated as alternate / future lane
- the current outer wrapper lowering is still incomplete for the verifier body

Remaining tasks:

- continue lowering the canonical outer circuit into canonical R1CS
- finish the non-native BN254 pairing-core lowering in that lane
- only once that lowering is sound and complete, promote the Arkworks R1CS lane
  to a first-class backend choice

Primary location:

- `crates/wrapper-circuits/src/r1cs/`
- `crates/wrapper-circuits/src/outer/r1cs.rs`

This is **not** the main blocker today.

### 4. Finalize the Real Outer Backend Choice

Deliverable:

- one concrete backend implementation that can actually run setup/prove/verify
  for the canonical outer Halo2/Midnight circuit

Current status:

- the trait exists
- the selected lane exists
- the remaining blocker is still the missing concrete prover / serializer path

Remaining tasks:

- choose the real proving stack for the canonical Halo2/Midnight outer circuit
- keep all stack-specific types contained behind the backend implementation
- preserve canonical circuit identity and artifact compatibility while using the
  direct Halo2/Midnight outer path

Primary location:

- `wrapper-backends/src/outer.rs`

This is the main blocker today.

### 5. Finish the Outer Circuit Input Adapter

Deliverable:

- a complete adapter from `WrapperExecutionPackage` to the exact witness/config
  shape needed by the real outer backend

Current status:

- mostly implemented for planning/adaptation

Remaining tasks:

- keep the current parsing/adaptation path strict
- add whatever extra proving inputs the real backend requires
- ensure all failures remain explicit:
  - missing proof payload
  - malformed VK
  - arity mismatch
  - unsupported statement layout
  - unsupported backend mode

Primary location:

- `wrapper-backends/src/outer.rs`

### 6. Replace the Placeholder VK Skeleton With Real Setup Output

Deliverable:

- a real `wrapper-verification-key.json`

Current status:

- planned shape exists
- real setup output does not

Remaining tasks:

- run real setup for the chosen backend
- serialize the produced VK into the agreed output shape
- verify emitted fields match the current contract:
  - `protocol`
  - `curve`
  - `nPublic`
  - `IC`
  - point encoding conventions

Primary location:

- `wrapper-backends`
- `wrapper-core/src/output.rs`

### 7. Replace `proof: null` With a Real Outer Proof

Deliverable:

- a real `wrapper-proof.json`

Current status:

- not implemented

Remaining tasks:

- run the real prover on a `.circom`-origin inner bundle
- serialize the produced proof into the agreed output shape
- verify emitted fields match the current contract:
  - key names
  - point encoding
  - protocol / curve labels

Primary location:

- `wrapper-backends`

### 8. Add Real Outer Verification

Deliverable:

- one path that verifies the produced outer proof against the produced outer VK
  and the exported outer public inputs

Current status:

- trait shape exists
- real produced bundle verification does not

Remaining tasks:

- add backend-level verification for the produced bundle
- use exactly the same ordered public inputs as `wrapper-public.json`
- make verification part of the acceptance path, not an optional afterthought

Primary location:

- `wrapper-backends`
- `wrapper-tests`

### 9. Promote the Real `.circom` Fixture Lane to a First-Class Integration Test

Deliverable:

- one reusable end-to-end integration test over a real `.circom`-origin proof

Current status:

- planning/stub coverage exists
- real produced outer bundle coverage does not

Remaining tasks:

- keep fixture loading generic through `ArtifactSetLoader`
- allow the fixture layer to provide semantic public-input names
- validate the full real chain:
  - load BN254 bundle
  - build wrapper package
  - run the real outer backend
  - produce outer bundle
  - verify outer bundle

Current recommended real fixture:

- Semaphore

Recommended future contrast fixture:

- a smaller non-ECC `.circom` circuit

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

Remaining tasks:

- load inner bundle
- build wrapper package
- run real setup/prove/verify as requested
- write:
  - `wrapper-proof.json`
  - `wrapper-public.json`
  - `wrapper-verification-key.json`
- optionally print the canonical circuit identity used for the run

## Validation Matrix

Minimum validation before claiming end-to-end completion:

- unit tests for:
  - output-model serialization
  - backend setup/prove/verify adapters
  - statement/public-input arity checks
  - real produced VK/proof shape validation
- integration tests for:
  - canonical small R1CS + Arkworks smoke coverage
  - real `.circom` fixture such as Semaphore
  - full produced outer bundle verification
- CLI smoke test for:
  - full outer-proof generation
  - outer-proof verification

## Recommended Execution Order

1. Finalize the real outer backend choice for the canonical Halo2/Midnight outer circuit.
2. Finish the outer circuit input adapter for the chosen backend.
3. Implement real setup and emit a produced verification key.
4. Implement real proving and emit a produced proof.
5. Implement real verification against produced VK + public inputs.
6. Promote the Semaphore lane to a real end-to-end integration test.
7. Add the CLI command for real outer generation.

## Immediate Blocker Summary

Today the main blocker is:

- missing concrete prover/serializer support for the canonical outer
  Halo2/Midnight circuit inside the selected outer backend lane

Everything else now hangs off that decision and implementation.

## Non-Goals For This Plan

- moving application-specific naming into generic backend parsing
- pretending the current placeholder output is already a real proof
- reimplementing QAP or Groth16 internals manually
- changing canonical R1CS identity to match a backend serialization format
- broadening the repo into a general-purpose proof ecosystem before the real
  end-to-end wrapper path exists
