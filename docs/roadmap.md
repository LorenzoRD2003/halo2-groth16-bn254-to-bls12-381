# Roadmap

## When To Read This Document

Read this document when the main question is "should this work exist yet?" or
"what stage does this belong to?".

Use it together with:

- `README.md` for the fastest high-level repo snapshot
- `AGENTS.md` for binding scope boundaries and code-touching rules
- `docs/architecture.md` for crate ownership and implementation placement

## Initialization

Status: completed.

Goals:

- establish workspace structure
- document architectural boundaries
- create honest placeholder types and traits
- provide a small CLI for diagnostics and validation
- make the repository compile and test cleanly

Non-goals:

- any cryptographic implementation
- Halo2 circuit logic beyond the narrow Week 1 foundation
- backend adapters beyond scaffolding

## Stage 1

Status: current phase.

Goals:

- introduce the first real Halo2 dependency decisions
- land circuit-backed BN254 `fp add` / `fp mul`
- land circuit-backed BN254 `fp2` arithmetic
- land circuit-backed BN254 `fp6` arithmetic
- land circuit-backed BN254 `fp12` arithmetic
- land minimal BN254 G1 addition and on-curve enforcement
- land minimal BN254 G2 affine representation, negation, and on-curve enforcement
- land Miller-path BN254 G2 `double_with_line` / `mixed_add_with_line` extraction with a clear sparse Fp12-facing boundary
- land narrow Miller-loop accumulation over extracted BN254 G2 line coefficients with the real fixed BN254 optimal-ate prepared-step driver
- land narrow final exponentiation over that Miller-loop output, still without widening into a verifier-facing full pairing API
- land a narrow verifier-shaped pairing-product check that multiplies Miller outputs first and applies exactly one shared final exponentiation
- land sparse-specialized BN254 Miller accumulator line consumption for the current D-twist `(ell_0, ell_w, ell_vw)` layout
- land the first narrow Groth16 BN254 verifier slice: real snarkjs proof/VK parsing, IC linear combination, verifier-equation reduction to one pairing-product check, and end-to-end valid/invalid regression coverage
- measure real layout/row cost for the Week 1 primitives
- keep host/reference BN254 tower arithmetic centralized rather than duplicated across modules
- keep measured primitive metadata centralized so CLI reporting and benchmark-info stay in sync
- refine outer circuit configuration boundaries
- define sharper interfaces for normalized proof and VK inputs

Still excluded unless explicitly planned:

- broad public pairings
- full multi-pairing / verifier-facing pipeline beyond the current narrow pairing-check slice
- G2 subgroup checks
- broad scalar multiplication
- generalized Groth16 verifier frameworks beyond the first narrow BN254 slice
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

- broader Groth16 BN254 verifier decomposition beyond the landed narrow slice
- integration of verifier subcomponents into the outer Halo2 circuit
- soundness-oriented tests and richer fixture strategy
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
