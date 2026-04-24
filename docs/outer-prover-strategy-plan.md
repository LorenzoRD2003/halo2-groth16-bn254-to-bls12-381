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
   - `OuterProofBackend`
   - package/adaptation/planning-oriented

2. Direct canonical outer-circuit surface in `wrapper-backends/src/outer.rs`
   - `CanonicalOuterCircuitProofBackend`
   - centered on `OuterWrapperCircuit`
   - intended to host the real direct setup / prove / verify path

Current state of the direct surface:

- planning over the canonical outer circuit is implemented
- construction and synthesis-readiness checks are implemented
- real setup is now wired through `midnight_proofs` keygen
- real proving is now wired through `midnight_proofs::plonk::create_proof(...)`
- real verification is now wired through `midnight_proofs::plonk::prepare(...)`
  plus PCS guard finalization

## Why This Is The Current Priority

The canonical outer wrapper circuit already embeds the BN254 non-native
verifier logic needed for the wrapper experiment.

The remaining blockers for a usable real outer proof flow are therefore:

- performance and ergonomics of the expensive direct proving lane
- broader automation / CI coverage around that lane

Those are smaller and more direct blockers than:

- fully lowering the entire non-native BN254 pairing core into canonical R1CS

## Selected Stack

Current concrete direct stack:

- setup API: `midnight_proofs::plonk::keygen_vk_with_k(...)`
- PCS: `midnight_proofs::poly::kzg::KZGCommitmentScheme<midnight_curves::bn256::Bn256>`
- serializer: `serde` JSON carrying hex-encoded `SerdeFormat::Processed` payloads
- honest direct-artifact curve label: `bn254`

Now implemented on top of that stack:

- exact proof-generation API wiring
- exact verification API wiring
- honest produced-proof JSON payload

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
