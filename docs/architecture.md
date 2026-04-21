# Architecture

## Purpose

This repository is structured for staged development of a Halo2-based wrapper around Groth16 BN254 proofs. The current repository state now includes a circuit-backed BN254 primitive layer: Week 1 delivered Fp and minimal G1 support, and Week 2 has started with a first Fp2 slice plus a minimal G2 affine representation/on-curve slice. It still does not implement G2 arithmetic, pairings, verifier logic, or a production wrapper circuit.

## Intended Data Flow

The expected long-term flow is:

1. Backend adapters load or normalize external artifacts such as proof metadata, verification key material, or ecosystem-specific formats.
2. `wrapper-core` expresses stable domain concepts for those artifacts, wrapper configuration, capability declarations, and execution boundaries.
3. `wrapper-circuits` consumes domain-level configuration and normalized metadata to construct Halo2-facing circuit descriptions.
4. The CLI or future orchestration layers coordinate configuration loading, validation, inspection, and eventually proof-related workflows.

The current implementation includes enough BN254 arithmetic to validate Week 1 interfaces, circuit wiring, and layout measurements, while still stopping well short of a wrapper verifier.

## Why `wrapper-core` Stays Domain-Oriented

`wrapper-core` is the anchor for stable concepts that should outlive changes in circuit frameworks or backend adapters. Keeping it mostly independent from Halo2 has several advantages:

- domain modeling can evolve without dragging proving-system dependencies into every consumer
- CLI validation and backend parsing can remain lightweight
- tests can exercise core logic without requiring cryptographic crates
- future rewrites of circuit internals do not force broad public API churn

## Why Circuits and Backends Are Separate

Circuit code and backend integration change for different reasons.

`wrapper-circuits` will eventually own:

- Halo2 circuit composition
- chip and gadget organization
- layout and witness-shape planning
- outer wrapper circuit boundary definitions
- the BN254 foreign-field layer introduced in Week 1 and extended in early Week 2 with Fp2
- the BN254 G1 abstraction layer introduced in Week 1
- the BN254 G2 affine representation layer introduced in Week 2

`wrapper-backends` will eventually own:

- artifact loading
- verification key ingestion
- proof metadata parsing
- compatibility adapters for other libraries and ecosystems

Separating these concerns prevents parser logic, serialization quirks, or artifact format churn from leaking into circuit modules.

## Halo2 Boundary Strategy

The project expects Halo2-specific code to live primarily in `wrapper-circuits`. Week 1 now uses `midnight-circuits` and `midnight-proofs` directly for a first real non-native BN254 layer, while keeping the supported surface intentionally small. This gives the project real circuit feedback without overcommitting to later-stage pairing or verifier APIs.

When Halo2 is introduced later:

- `wrapper-core` should still avoid direct dependence unless a boundary cannot be represented otherwise
- `wrapper-circuits` should absorb the proving-system integration surface
- `wrapper-backends` should remain focused on external artifact and ecosystem concerns

## BN254 Foreign-Field Layer

Week 1 adds an `AssignedFp` abstraction in `wrapper-circuits`, and Week 2 begins by layering `AssignedFp2` on top of it.

Current properties:

- Midnight-backed assigned BN254 base-field values
- circuit-backed `add`, `sub`, `neg`, `mul`, and `square`
- circuit-backed BN254 Fp2 values represented as `(c0, c1)` for `c0 + c1 * u`
- Fp2 `add`, `sub`, `neg`, `mul`, and specialized `square` expressed through the existing `AssignedFp` layer
- real row and layout measurements via `midnight_proofs::dev::cost_model`
- arkworks-backed randomized correctness tests

Current limitations:

- no Fp6, Fp12, or pairing-specific arithmetic yet
- no production-oriented optimization or custom layout tuning yet
- row and query reporting is real, but still only for the narrow implemented circuits

## BN254 G1 Abstraction Layer

Week 1 also adds an `AssignedG1` abstraction in `wrapper-circuits`.

Current properties:

- Midnight-backed assigned BN254 G1 points
- circuit-backed complete point addition
- coordinate-to-point construction with on-curve enforcement
- deterministic arkworks-backed correctness tests
- real layout metrics for the Week 1 G1 addition circuit

Current limitations:

- no public Week 1 MSM surface
- no subgroup-check or cofactor-clearing workflow yet
- no G2 or pairing support

## BN254 G2 Affine Layer

Week 2 adds a minimal `AssignedG2Affine` abstraction in `wrapper-circuits`.

Current properties:

- G2 affine points represented as `(x, y)` over `AssignedFp2`
- circuit-backed non-infinity assignment from Fp2 coordinates
- circuit-backed negation
- circuit-backed equality checks
- explicit twist on-curve validation against the BN254 G2 equation from arkworks
- real layout metrics for narrow `g2 on_curve` and `g2 neg` sanity circuits

Current limitations:

- no identity/infinity representation in this slice
- no G2 addition, doubling, projective formulas, or scalar multiplication
- no subgroup checks yet
- no pairing support

## Current Architectural Contracts

The current skeleton defines:

- wrapper phases and status reporting
- wrapper capabilities and implementation status markers
- repository configuration parsing and validation
- layout descriptors for future circuit inspection
- backend registry and artifact loader interfaces
- BN254 field, Fp2, G1, and minimal G2 affine foundations with real layout visibility

These contracts are intentionally conservative and meant to support staged development rather than predict final cryptographic APIs in detail.
