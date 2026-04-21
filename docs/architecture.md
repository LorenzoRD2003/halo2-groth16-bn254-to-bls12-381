# Architecture

## Purpose

This repository is structured for staged development of a Halo2-based wrapper around Groth16 BN254 proofs. The current repository state provides only architectural scaffolding and does not implement cryptographic functionality.

## Intended Data Flow

The expected long-term flow is:

1. Backend adapters load or normalize external artifacts such as proof metadata, verification key material, or ecosystem-specific formats.
2. `wrapper-core` expresses stable domain concepts for those artifacts, wrapper configuration, capability declarations, and execution boundaries.
3. `wrapper-circuits` consumes domain-level configuration and normalized metadata to construct Halo2-facing circuit descriptions.
4. The CLI or future orchestration layers coordinate configuration loading, validation, inspection, and eventually proof-related workflows.

The initialization phase implements only the configuration, metadata, and boundary definitions required for that flow.

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

`wrapper-backends` will eventually own:

- artifact loading
- verification key ingestion
- proof metadata parsing
- compatibility adapters for other libraries and ecosystems

Separating these concerns prevents parser logic, serialization quirks, or artifact format churn from leaking into circuit modules.

## Halo2 Boundary Strategy

The project expects Halo2-specific code to live primarily in `wrapper-circuits`. During initialization, Halo2 is not yet required as a dependency because no actual circuit implementation exists.

When Halo2 is introduced later:

- `wrapper-core` should still avoid direct dependence unless a boundary cannot be represented otherwise
- `wrapper-circuits` should absorb the proving-system integration surface
- `wrapper-backends` should remain focused on external artifact and ecosystem concerns

## Current Architectural Contracts

The current skeleton defines:

- wrapper phases and status reporting
- wrapper capabilities and implementation status markers
- repository configuration parsing and validation
- layout descriptors for future circuit inspection
- backend registry and artifact loader interfaces

These contracts are intentionally small and meant to support staged development rather than predict final cryptographic APIs in detail.

