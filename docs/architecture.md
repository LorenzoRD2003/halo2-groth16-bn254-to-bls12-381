# Architecture

## When To Read This Document

Read this document when you need crate ownership, data flow, proof-boundary
semantics, or clarity about which layer owns which concern.

If you only need a fast repository snapshot, start with `README.md`.
If you need binding task constraints or the fastest code-reading order, start
with `AGENTS.md`.

Fast path through this document:

1. `Purpose`
2. `Current System Shape`
3. `End-to-End Data Flow`
4. `Outer Statement and VK Binding`
5. `Crate Ownership`
6. the specific subsystem section that matches your task

## Purpose

This repository is structured for staged development of a Halo2/Midnight outer
proof system that wraps Groth16 BN254 proofs.

The current codebase is no longer just a primitive experiment. It now contains:

- a circuit-backed BN254 primitive layer
- a narrow but real Groth16 BN254 verifier slice
- artifact parsing and planning above `snarkjs`-shaped inputs
- a canonical outer wrapper circuit
- real direct `setup -> prove -> verify` lanes for hosted outer proofs
- a public outer-statement model that binds verification to a specific inner
  verification key

The repository is still intentionally narrow.

It does **not** currently aim to provide:

- a broad general-purpose pairing API
- a broad general-purpose Groth16 verifier framework
- generalized wrapper statement DSLs
- production-optimized non-native arithmetic everywhere
- a fully generalized outer-backend ecosystem

## Current System Shape

At a high level, the current system has four important layers:

1. external artifact ingestion
2. domain modeling and planning
3. canonical outer-circuit semantics
4. backend-specific setup/prove/verify materialization

The current happy-path flow is:

`snarkjs artifacts -> Groth16Bn254ArtifactBundle -> WrapperJob -> WrapperExecutionPackage -> OuterWrapperCircuit -> direct outer backend -> produced outer artifacts`

The repository also contains a canonical R1CS line, but it is still an
alternate or future backend path rather than the delivery-critical one.

## End-to-End Data Flow

### External Inputs

The repository currently expects inner proof artifacts in the standard
`snarkjs`-style triple:

- `proof.json`
- `public.json`
- `verification_key.json`

Those artifacts are normalized by `wrapper-backends`.

### Domain and Planning Layer

`wrapper-backends` builds a normalized
`Groth16Bn254ArtifactBundle`, which then feeds:

- `WrapperJob`
- `WrapperExecutionPackage`
- `WrapperExecutionResult`

This planning layer lives mostly in `wrapper-core`, with parsing and adaptation
owned by `wrapper-backends`.

### Canonical Outer-Circuit Layer

`wrapper-circuits` owns the canonical outer semantic circuit:

- semantic type: `OuterWrapperCircuit`
- canonical input: `OuterWrapperCircuitInput`

This is the single outer-circuit source of truth in the current repository
phase.

### Backend Materialization Layer

`wrapper-backends` owns setup/prove/verify materialization over that canonical
outer circuit.

Today there are two real direct outer host lanes:

- `MidnightDirectOuterBackendBn254Host`
- `MidnightDirectOuterBackendBls12Host`

Both are real backend lanes.
Neither should be treated as a neutral or abstract “default backend”.

Current lane policy:

- `BLS12-381` is the official outer lane
- `BN254` remains a compatibility/testing lane
- documentation, operator flows, and external integration planning should treat
  `BLS12-381` as the public-facing target

## Outer Statement and VK Binding

### What The Outer Proof Claims

The outer proof now claims three things:

1. the supplied Groth16 BN254 proof verifies against the supplied normalized
   verification key and ordered public inputs
2. the outer public statement mirrors the ordered inner public inputs
3. the witness-side inner verification key hashes to the public
   `vk_commitment` carried by the outer statement

This is a materially stronger claim than the older mirror-only statement.

### Statement Shape

The outer statement is no longer modeled as “just a flat vector”.

The semantic statement now contains:

- `mirrored_field_names`
- `mirrored_public_inputs`
- `vk_commitment`

The current Halo2 exposure path still needs a flat vector, so the semantic
statement also derives:

- flat `field_names`
- flat `public_inputs`

The flattening rule is:

1. mirrored public inputs in caller-supplied order
2. flattened public limbs of the semantic `vk_commitment`

Current commitment limb names are:

- `vk_commitment_limb_0`
- `vk_commitment_limb_1`
- and so on

### Commitment Primitive

The current inner verification-key commitment is **not** an ad hoc field fold.

It is now:

- a Poseidon x^5 based commitment
- defined over `BN254::Fq`
- computed from the canonical normalized Rust
  `Groth16Bn254VerifyingKey`
- stable across the two hosted outer lanes because it is defined over the
  semantic VK field, not over a host-lane-native hash domain

The canonical coordinate stream is:

1. `alpha_g1`
2. `beta_g2`
3. `gamma_g2`
4. `delta_g2`
5. `ic` in verifier order

Point flattening order:

- G1 as `(x, y)`, with identity encoded as `(0, 0)`
- G2 as `(x.c0, x.c1, y.c0, y.c1)`

Implementation entry point:

- `crates/wrapper-circuits/src/groth16/commitment.rs`

### Where The Binding Is Enforced

The binding is enforced inside the outer circuit semantics.

The circuit:

1. assigns the witness-side normalized inner VK as non-native BN254 values
2. recomputes the Poseidon-based VK commitment in-circuit
3. constrains that computed value to equal the semantic `vk_commitment`
4. then runs the narrow Groth16 verifier relation as before

This means the public VK binding is a hard circuit constraint, not just a
host-side planning convention.

## Why `wrapper-core` Stays Domain-Oriented

`wrapper-core` is the stable domain layer.

It should remain mostly independent from Halo2 because:

- package and planning contracts outlive circuit rewrites
- CLI and loader logic stay lightweight
- tests can exercise planning logic without full proving dependencies
- backend churn should not force broad public API churn

Today `wrapper-core` owns:

- named public-input views
- wrapper-job planning types
- wrapper execution-package types
- explicit wrapper statement modeling
- explicit verification-key commitment modeling
- expected wrapper output artifact shapes
- execution results for both stub and direct lanes

`wrapper-core` should not absorb:

- host-lane-specific proving behavior
- circuit chip details
- region/layout concerns
- parser quirks from external ecosystems

## Why `wrapper-circuits` and `wrapper-backends` Stay Separate

These two crates evolve for different reasons.

`wrapper-circuits` owns:

- canonical outer-circuit semantics
- host-lane wrappers around that semantic circuit
- the BN254 non-native primitive layer
- the Groth16 verifier slice
- the in-circuit VK binding check
- layout and R1CS-lowering related circuit logic

`wrapper-backends` owns:

- artifact loading
- external format parsing
- bundle normalization
- package adaptation into circuit inputs
- backend metadata
- setup/prove/verify materialization for concrete outer lanes

Keeping them separate prevents:

- artifact-format churn from leaking into circuit modules
- host-backend details from polluting domain contracts
- circuit ownership from becoming ambiguous across backend lanes

## Crate Ownership

### `wrapper-core`

Primary responsibilities:

- stable domain contracts
- package and execution modeling
- statement semantics at the domain level
- expected output-artifact modeling

Must not own:

- Halo2 chip logic
- witness assignment logic
- backend-specific serialization internals

### `wrapper-circuits`

Primary responsibilities:

- Halo2-facing circuit logic
- canonical `OuterWrapperCircuit`
- BN254 non-native arithmetic
- Groth16 verifier circuit semantics
- in-circuit VK commitment recomputation
- hosted circuit wrappers for outer lanes

Must not own:

- raw `snarkjs` parsing
- filesystem artifact loading
- backend-specific artifact packaging

### `wrapper-backends`

Primary responsibilities:

- proof/VK/public-input parsing
- normalized bundle construction
- direct outer backend surfaces
- backend metadata and artifact serialization contracts

Must not own:

- a second competing outer circuit definition
- broad domain ownership better suited to `wrapper-core`

### `wrapper-cli`

Primary responsibilities:

- operator and developer workflows
- inspection and planning commands
- direct execution commands
- user-facing summaries of package and statement shape

### `wrapper-tests`

Primary responsibilities:

- committed fixtures
- integration helpers
- cross-crate regression coverage

## Canonical Outer Circuit

The canonical outer circuit remains:

- file root: `crates/wrapper-circuits/src/outer/`
- semantic type: `OuterWrapperCircuit`

This remains the only circuit source of truth for the outer wrapper in the
current phase.

`OuterWrapperCircuit` is the semantic object.
The actual host-lane `Circuit<F>` implementations live in:

- `HostedOuterWrapperCircuitBn254`
- `HostedOuterWrapperCircuitBls12`

This split is intentional:

- the semantic statement should not be redefined per host lane
- hosted proving wrappers should not become the semantic source of truth

## Outer Backend Lanes

### Real Direct Lanes

The repository currently supports two real direct outer lanes:

- `MidnightDirectOuterBackendBn254Host`
- `MidnightDirectOuterBackendBls12Host`

Both support:

- setup
- prove
- verify
- split `prove-trace` / `prove-finalize`

However, they do not have the same product weight:

- `MidnightDirectOuterBackendBls12Host` is the official lane
- `MidnightDirectOuterBackendBn254Host` is retained for compatibility,
  regression, and comparative profiling

### Planned Compatibility Lane

The planning-only lane remains:

- `PlannedHalo2OuterBackend`

It exists to materialize honest artifact contracts and planning expectations
without claiming real proof production.

### Naming Contract

There is no longer a repository-local alias that should be read as
“the direct outer backend”.

Code should use explicit lane names.

That matters because:

- the repo maintains two real outer host lanes
- the statement semantics are shared across them
- backend naming should not imply a false architectural default

## BN254 Primitive and Verifier Surface

The current implemented BN254 line includes:

- `Fp`
- `Fp2`
- `Fp6`
- `Fp12`
- G1
- narrow G2 affine / projective support
- Miller-path line extraction
- final exponentiation
- narrow pairing-product check
- narrow Groth16 BN254 verification

The repository still does **not** expose:

- broad G2 subgroup-check APIs
- broad G2 scalar multiplication APIs
- broad public pairing APIs
- a generalized verifier framework

The design remains verifier-shaped and intentionally narrow.

## Hosted Outer Lanes vs Inner Verifier Field

The inner verifier remains BN254 in both hosted outer lanes.

That means the implementation must distinguish between:

- the field and curve family of the **inner proof system**
- the host field of the **outer Halo2 proof**

This distinction is why the VK commitment is defined over semantic BN254
coordinates rather than over one host-lane-native hash field.

## Canonical R1CS Line

The repository also contains a canonical R1CS line under
`crates/wrapper-circuits/src/r1cs/`.

Current role of that line:

- deterministic lowering target
- canonical identity-hash source
- zkInterface-style export bridge
- alternate or future backend direction

Current non-role:

- it is **not** the delivery-critical path for the current direct outer flow

The practical outer delivery path still runs through the canonical Halo2 /
Midnight circuit and the direct backend lanes.

## Current Architectural Contracts

The current architecture assumes:

- one canonical outer semantic circuit
- explicit outer statement semantics
- explicit public VK binding
- backend lanes materialize the same semantic statement on different hosts
- host-lane specifics stay in backend and hosted-circuit surfaces
- external artifact parsing stays out of circuit modules

In other words:

- `wrapper-core` defines what the system means
- `wrapper-circuits` defines how that meaning is enforced in-circuit
- `wrapper-backends` defines how concrete artifact ecosystems and hosted proof
  lanes plug into that circuit

## Practical Reading Routes

If you need outer statement semantics and VK binding:

1. `crates/wrapper-circuits/src/groth16/commitment.rs`
2. `crates/wrapper-circuits/src/outer/statement.rs`
3. `crates/wrapper-circuits/src/outer/input.rs`
4. `crates/wrapper-circuits/src/outer/semantics.rs`

If you need direct outer backend lane context:

1. `crates/wrapper-backends/src/outer/direct/mod.rs`
2. `crates/wrapper-backends/src/outer/direct/adaptation.rs`
3. `crates/wrapper-backends/src/outer/direct/proving.rs`
4. `docs/outer-prover-strategy-plan.md`

If you need package/planning context:

1. `crates/wrapper-backends/src/groth16.rs`
2. `crates/wrapper-core/src/package.rs`
3. `crates/wrapper-core/src/execution.rs`
4. `crates/wrapper-core/src/output.rs`

If you need committed integration coverage:

1. `crates/wrapper-tests/src/lib.rs`
2. `crates/wrapper-backends/src/outer/tests.rs`
3. `crates/wrapper-circuits/src/outer/tests.rs`
