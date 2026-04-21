# Roadmap

## Initialization

Status: current phase.

Goals:

- establish workspace structure
- document architectural boundaries
- create honest placeholder types and traits
- provide a small CLI for diagnostics and validation
- make the repository compile and test cleanly

Non-goals:

- any cryptographic implementation
- Halo2 circuit logic beyond placeholders
- backend adapters beyond scaffolding

## Stage 1

Goals:

- introduce the first real Halo2 dependency decisions
- refine outer circuit configuration boundaries
- define sharper interfaces for normalized proof and VK inputs
- begin layout-oriented planning for the wrapper circuit

Still excluded unless explicitly planned:

- pairings
- Groth16 verifier logic
- production-ready backend support

## Later Pairing Work

Potential goals:

- foreign field arithmetic design
- ECC representation strategy
- pairing-related gadget research
- cost modeling and constraint budgeting

This stage should be preceded by explicit design notes and likely additional ADRs.

## Later Wrapper Verifier Work

Potential goals:

- Groth16 BN254 verifier decomposition
- integration of verifier subcomponents into the outer Halo2 circuit
- soundness-oriented tests and fixture strategy
- performance and proof-size analysis

## Possible Cardano Integration

Potential goals:

- integration constraints relevant to Cardano or IOG-adjacent workflows
- artifact packaging and serialization expectations
- ecosystem-specific operational tooling

This is exploratory and should remain decoupled from the core architecture until requirements are concrete.

## Possible Semaphore Migration Case Study

Potential goals:

- apply the wrapper to a Semaphore-like migrated circuit use case
- validate assumptions about artifact ingestion and wrapper ergonomics
- collect implementation lessons from a real application-shaped example

This case study belongs after core cryptographic machinery exists.

