# Outer Prover Strategy Plan

## Purpose

This document records the current proving-strategy choice for the canonical
outer wrapper circuit.

Its job is to make one thing explicit for future implementors:

- the first real end-to-end wrapper flow should prioritize proving the
  canonical `OuterWrapperCircuit` directly in the Halo2/Midnight ecosystem
- the canonical R1CS line remains important, but is currently an alternate /
  future backend path

## Current Decision

Preferred first real backend:

- prove the canonical `OuterWrapperCircuit` directly
- keep the circuit authored in Halo2/Midnight
- serialize the resulting proof / VK / public artifacts honestly according to
  the real proof-system family chosen by that backend

This means:

- do not force the first real backend through canonical R1CS if a direct
  Halo2/Midnight prover path is available sooner
- do not pretend a PLONKish / Halo2 proof is Groth16 if it is not
- keep `R1csCircuit::identity_hash()` as a strategic identity source for the
  alternate backend lane, but not as the first delivery blocker

## Current Backend Surfaces

The repo now has two complementary backend surfaces:

1. Package-oriented outer backend surface in `wrapper-backends/src/outer.rs`
   - `OuterGroth16Backend`
   - package/adaptation/planning-oriented

2. Direct canonical outer-circuit surface in `wrapper-backends/src/outer.rs`
   - `CanonicalOuterCircuitProofBackend`
   - centered on `OuterWrapperCircuit`
   - intended to host the real direct setup / prove / verify path

Current state of the direct surface:

- planning over the canonical outer circuit is implemented
- construction and synthesis-readiness checks are implemented
- real setup / prove / verify are not wired yet

## Why This Is The Current Priority

The canonical outer wrapper circuit already embeds the BN254 non-native
verifier logic needed for the wrapper experiment.

The remaining blocker for a real outer proof flow is therefore:

- a real prover / serializer backend for that canonical circuit

That is a smaller and more direct blocker than:

- fully lowering the entire non-native BN254 pairing core into canonical R1CS

## What Still Needs To Be Decided

The real direct backend still requires one concrete proving stack choice:

- exact setup API
- exact proving API
- exact verification API
- verification-key serialization shape
- proof serialization shape
- public-input artifact serialization

## Constraints

Any chosen backend should preserve these rules:

- the outer circuit remains canonical in `wrapper-circuits/src/outer/`
- backend-specific types stay behind `wrapper-backends`
- output artifacts must be named and shaped honestly
- public-input ordering must remain `WrapperStatement` order
- application-specific semantic names remain in fixture/domain layers

## Follow-On Relationship To Canonical R1CS

The canonical R1CS line should continue in parallel because it provides:

- deterministic circuit identity
- alternate backend path
- future CRS-binding foundation

But it should not currently block the first real outer-proof delivery.
