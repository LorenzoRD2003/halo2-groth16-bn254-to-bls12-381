# Canonical R1CS Backend Status

## Purpose

This document records the current status of the canonical R1CS line that lives
under `crates/wrapper-circuits/src/r1cs/`.

Its job is to answer two questions clearly:

1. what has already been implemented in the canonical R1CS path
2. how that path should be treated relative to the real `.circom` -> outer
   wrapper delivery path

## Current Positioning

The canonical R1CS line is now a **real implemented subsystem**, but it should
currently be treated as:

- an **alternate backend lane**
- a **future/progressive backend path**
- a **source of canonical circuit identity**

It is **not** the current critical path for shipping the first real
`.circom`-origin wrapper flow.

For the real wrapper flow, the current critical path remains:

- canonical Halo2/Midnight outer circuit
- real prover / serializer support for that circuit
- produced outer proof / VK artifacts

## What Exists Today

### Canonical R1CS Core

Implemented under `crates/wrapper-circuits/src/r1cs/`:

- canonical R1CS model
- canonical linear-combination normalization
- deterministic constraint ordering
- deterministic Halo2-style cell identity
- equality / copy handling via deterministic variable unification
- explicit metadata boundary for future Halo2/Midnight extraction
- explicit public-input ordering via `public_index`
- canonical byte encoding
- `R1csIdentityHash`
- internal zkInterface bridge export

### Arkworks Adapter

Also implemented:

- Arkworks adapter from canonical `R1csCircuit` to
  `ark_relations::r1cs::ConstraintSynthesizer`
- Arkworks setup / prove / verify helpers on a small canonical R1CS smoke
  circuit

This proves that the canonical R1CS path is viable as a backend surface.

### Outer Wrapper R1CS Lowering Progress

Implemented under `crates/wrapper-circuits/src/outer/r1cs.rs`:

- `OuterStatementExposure` slice: lowered
- `Groth16IcAccumulator` slice: lowered for the currently modeled scalar
  schedule/wiring layer
- `VerifierResultAssertion` slice: lowered
- `Groth16PairingProductCheck` slice: prepared with deterministic extracted
  inputs and host-side reference behavior

This means the outer-wrapper R1CS lowering is no longer a blank placeholder,
but it is still incomplete.

## What Is Still Missing

The canonical R1CS line still lacks a **sound non-native BN254 R1CS layer**
capable of carrying the full pairing core.

In practice, the main remaining blocker is:

- canonical R1CS lowering of the BN254 pairing-product check

That requires explicit non-native R1CS support for:

- `Fq`
- `Fq2`
- `Fq6`
- `Fq12`
- the Miller / final-exponentiation / pairing-product pipeline

Some non-native tower scaffolding now exists under:

- `crates/wrapper-circuits/src/r1cs/non_native/`

But that scaffolding is not yet a sound full R1CS backend for the pairing core.

## Why This Is Not The Critical Path

The original project goal was never "R1CS first at all costs".
The original goal was:

- verify a real Groth16 BN254 proof inside an outer circuit
- then emit a real outer Groth16 BLS12-381 proof

The existing Halo2/Midnight BN254 primitive layer already implements the
non-native arithmetic and pairing logic needed for that verifier.

Therefore:

- the shortest path to a real wrapper artifact flow is still the canonical
  Halo2/Midnight outer circuit plus a real prover/serializer backend
- the canonical R1CS path remains strategically important, but should currently
  be treated as an alternate/future backend lane rather than the delivery
  critical path

## Recommended Project Stance

For now:

- keep `R1csCircuit::identity_hash()` as the canonical identity source
- continue documenting and testing the R1CS lane honestly
- do not claim the R1CS backend is the primary execution path for the real
  outer wrapper flow yet
- prioritize the real Halo2/Midnight outer proving path for delivery

Later:

- continue porting the non-native BN254 tower and pairing core into canonical
  R1CS
- once the outer wrapper has a full canonical R1CS lowering, the Arkworks R1CS
  backend can become a first-class concrete backend choice

## Short Summary

Today the canonical R1CS line is:

- real
- useful
- strategically important

But for the real `.circom` wrapper path, it is still:

- alternate
- incomplete
- not the immediate critical path
