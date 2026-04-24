# Plutus / Aiken Integration Plan

## Purpose

This document captures the integration plan between:

- this repository, which owns the `.circom` / `snarkjs` Groth16 BN254 ingestion
  path plus the canonical `OuterWrapperCircuit`
- `plutus-halo2-verifier-gen`, which owns Aiken / Plinth verifier generation
  for Halo2-style circuits

The goal is a practical end-to-end developer flow:

```text
Circom / snarkjs Groth16 BN254 artifacts
  -> wrapper planning + canonical outer circuit
  -> outer Halo2 / Midnight proof on a Cardano-compatible host lane
  -> Aiken validator generation
```

This plan is intentionally integration-oriented. It does not redefine the core
architecture of either repository; it describes how to make them compose.

## Current State

## What This Repo Already Provides

- `snarkjs` Groth16 BN254 parsing and bundle loading
- domain-level `WrapperJob` and `WrapperExecutionPackage` planning
- a canonical `OuterWrapperCircuit`
- a direct backend lane with real `setup -> prove -> verify`
- CLI entry points for planning and direct execution

In short, this repo already provides the "inner proof ingestion + outer circuit
ownership" half of the product.

## What `plutus-halo2-verifier-gen` Already Provides

- extraction of verifier structure from Halo2-style circuits
- Aiken / Plinth verifier code generation
- a dedicated Midnight extraction lane
- Cardano-friendly transcript and verifier-generation workflow

In short, that repo already provides the "turn a compatible Halo2 verifier into
an Aiken validator" half of the product.

## Main Compatibility Gap

The two repositories do not yet line up on the same outer proving lane.

Current state:

- this repo's direct outer backend is honest `halo2-plonkish / bn254`
- `plutus-halo2-verifier-gen` is built around verifier generation for a
  BLS12-based Halo2 / Midnight lane, including its Midnight extraction path

That means the current integration problem is not "how do we parse Circom?".
It is:

> how do we make the canonical outer circuit from this repo available on a
> proving lane that `plutus-halo2-verifier-gen` can export to Aiken?

## Target Integration Shape

The desired architecture is:

```text
Inner proof:
  Groth16 BN254 from Circom / snarkjs

Inner verifier logic:
  BN254 verifier semantics

Outer circuit host lane:
  Halo2 / Midnight on a Cardano-compatible BLS12-based lane

Verifier export:
  plutus-halo2-verifier-gen -> Aiken validator
```

Key principle:

- keep the inner proof format and verifier semantics BN254
- change only the host proving lane of the outer proof

This preserves the product goal:

```text
any Circom BN254 proof
  -> wrapped
  -> exported as an Aiken validator flow
```

## Phase 1: Abstractions To Add In This Repo

The first step is not a curve switch. The first step is to separate concerns so
the curve switch becomes local and maintainable.

### 1. Separate Inner-Verifier Semantics From Outer Host Lane

Today, parts of the outer lane are still implicitly tied to the current host
field / proving lane.

Introduce explicit abstraction boundaries for:

- inner proof artifact family
- inner verifier semantics
- outer host field / transcript / PCS family
- outer artifact serialization contract

Recommended direction:

- define an `OuterHostFlavor` trait or equivalent type family
- define an `InnerVerifierFlavor` trait or equivalent type family
- make `OuterWrapperCircuit` parameterizable over the host lane boundary, even
  if only one concrete implementation exists initially

The point is not genericity for its own sake.
The point is to avoid rewriting the circuit when the outer lane changes from
BN254-hosted to a BLS12-hosted lane.

### 2. Isolate The Current Native-Field Coupling

Today the outer circuit is hosted on the current `NativeField` alias.

Refactor toward:

- one module that owns "host field / host transcript / host PCS" choices
- one module that owns "BN254 verifier semantics as non-native logic"

This should make it possible to say:

- "verify BN254 inside the circuit"
- without simultaneously saying
- "therefore the outer proof itself must be hosted on BN254"

### 3. Introduce Explicit Outer Backend Capabilities

The backend contract should expose the host lane as a first-class concept.

Recommended additions:

- stable backend metadata for host curve / PCS / transcript
- a reusable adapter boundary between `WrapperExecutionPackage` and the
  concrete host-lane circuit input
- typed serialization helpers for proof / VK / verifier params, independent
  from one specific lane

This allows:

- current BN254-hosted direct lane to keep working
- future BLS12-hosted lane to be added as a sibling, not a rewrite

### 4. Preserve The Current Lane As A Compatibility Reference

Do not delete the current direct lane immediately.

Keep:

- the current direct backend as the reference implementation for the current
  repo state
- tests that validate the existing BN254-hosted path

Reason:

- it gives a working baseline during the refactor
- it preserves an implementation oracle when the BLS12-hosted lane is added

## Phase 2: Concrete Modifications In This Repo

### 1. Add A Host-Lane Abstraction Module

Create a module dedicated to the outer proving host, for example:

```text
crates/wrapper-circuits/src/outer/host/
```

Possible contents:

- trait(s) describing host field / transcript / PCS choices
- config type(s) for current and future host lanes
- current BN254-hosted implementation
- placeholder BLS12-hosted implementation shell

### 2. Refactor `OuterWrapperCircuit`

Target direction:

- `OuterWrapperCircuit` should remain the canonical semantic circuit
- host-lane-specific proving details should move out of the semantic circuit
  definition and into the host-lane abstraction / backend layer

Meaning:

- the circuit continues to express "verify Groth16 BN254 and expose the outer
  statement"
- the host lane decides how that is proved

### 3. Refactor `wrapper-backends/src/outer.rs`

Introduce a structure like:

- one package-oriented orchestrator trait
- one host-lane-specific backend implementation per supported outer lane

Near-term target set:

- `MidnightDirectOuterBackendBn254Host`
- `MidnightDirectOuterBackendBls12Host` (new)

Even if the names differ, the idea is:

- current lane remains
- future lane is additive

### 4. Keep The CLI Surface Stable

The CLI should eventually let the caller choose the outer host lane.

Suggested direction:

- add a `--outer-host` or `--backend` flag
- default it conservatively
- make the output include the selected backend in the JSON result

This keeps the UX extensible without multiplying commands unnecessarily.

## Phase 3: Concrete Modifications In `plutus-halo2-verifier-gen`

### 1. Add A "Wrapper Circuit Consumer" Entry Surface

The other repo should gain a dedicated entry point for consuming this repo's
wrapper lane, rather than forcing the integration through ad hoc examples.

Suggested form:

- a library API that accepts:
  - params
  - VK
  - public-input layout
  - optional sample proof(s)
- a CLI command for generating an Aiken verifier from those inputs

### 2. Accept The Future BLS12-Hosted Wrapper Lane

The integration should target the future BLS12-hosted outer lane from this
repo, not the current BN254-hosted one.

That means the bridge code in `plutus-halo2-verifier-gen` should consume:

- the future BLS12-hosted Midnight-compatible verification key type
- the matching params / verifier params
- the matching transcript convention

The current extractor code already points in that direction.

### 3. Add A Thin Adapter Instead Of Rewriting Extraction Logic

Preferred approach:

- keep `plutus-halo2-verifier-gen`'s Aiken generation logic intact
- add a wrapper-specific adapter layer that maps this repo's produced outer
  artifacts / verifier objects into the types its generator already expects

Avoid:

- forking the generator path for one project-specific circuit
- duplicating the extraction / emitter logic

### 4. Add Cross-Repo Fixture Tests

Recommended tests inside `plutus-halo2-verifier-gen` once the bridge exists:

- canonical wrapper smoke fixture
- Semaphore wrapper fixture
- generated verifier round-trip against the produced proof / public inputs

These should be explicit integration tests, not hidden example scripts only.

## Recommended CLI Product Shape

The long-term product should feel like one coherent workflow.

### Phase-A CLI In This Repo

This repo should keep owning:

- inner artifact ingestion
- wrapper planning
- wrapper package creation
- outer proving

Useful commands here:

```text
wrapper-cli inspect-groth16-bundle
wrapper-cli plan-wrapper-job
wrapper-cli export-wrapper-package
wrapper-cli execute-wrapper-direct --backend <host-lane>
```

### Phase-B CLI In `plutus-halo2-verifier-gen`

That repo should own:

- verifier extraction
- Aiken / Plinth emission

Suggested command:

```text
cargo run --bin plutus-halo2-verifier-gen -- \
  generate-aiken-from-wrapper \
  --vk <outer-vk-artifact> \
  --proof <outer-proof-artifact> \
  --public <outer-public-artifact> \
  --backend midnight-bls12-wrapper \
  --out-dir <dir>
```

### Phase-C End-To-End UX

Once both halves exist, the intended UX is:

```text
1. wrapper-cli execute-wrapper-direct \
     --proof proof.json \
     --public public.json \
     --vk verification_key.json \
     --backend midnight-bls12-wrapper \
     --output wrapper-result.json

2. plutus-halo2-verifier-gen generate-aiken-from-wrapper \
     --result wrapper-result.json \
     --out-dir ./generated-aiken
```

This keeps responsibilities clean:

- this repo owns the wrapper
- the other repo owns the Aiken validator generation

## Why Not Integrate Via Current JSON Alone

Do not treat the current JSON artifacts from this repo as the final integration
surface yet.

Reason:

- they are honest and useful
- but they reflect the current BN254-hosted direct lane
- the target Aiken export path wants the future BLS12-hosted lane

So the best immediate use of the current JSON contract is:

- as a design guide
- as a shape/protocol reference
- not as the final plug-in surface

## Migration Order

Recommended order of work:

1. Add host-lane abstractions in this repo.
2. Preserve the current BN254-hosted direct lane.
3. Introduce a BLS12-hosted outer lane in this repo.
4. Make that lane produce the verifier objects / artifacts expected by the
   generator repo.
5. Add a wrapper-specific consumer command in `plutus-halo2-verifier-gen`.
6. Validate on the canonical fixture.
7. Validate on Semaphore.
8. Expose the full product as a documented CLI flow.

## Non-Goals

This plan does not aim to:

- replace the canonical `OuterWrapperCircuit` with a second competing circuit
- force the current BN254-hosted lane to disappear immediately
- turn the current R1CS line into the critical path
- make Aiken generation own Circom parsing directly

## Practical Summary

The integration is viable if approached as:

- this repo owns the wrapper semantics and wrapper proving lane
- `plutus-halo2-verifier-gen` owns the Aiken validator generation
- both repos meet at a BLS12-hosted Halo2/Midnight-compatible outer verifier
  boundary

The immediate next step is not "call the other repo from today's CLI".
The immediate next step is:

> make the outer lane in this repo extensible enough that a BLS12-hosted
> implementation can be added cleanly.
