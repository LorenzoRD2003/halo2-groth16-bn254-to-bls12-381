# AGENTS.md

## Project Purpose

This repository is a Rust workspace for a staged research and engineering effort around a Halo2-based wrapper that may eventually verify Groth16 BN254 proofs inside an outer Halo2 proof system.

The repository currently exists to establish architecture, crate boundaries, documentation, and workflow. It does not yet implement cryptographic logic.

## Current Phase and Scope Boundaries

Current phase: initialization and workspace bootstrap.

In scope:

- Workspace structure
- Crate manifests
- Domain interfaces and configuration models
- Placeholder circuit and backend abstractions
- Developer CLI
- Documentation and contributor guidance
- Minimal tests for scaffold behavior

Out of scope:

- Foreign field arithmetic
- ECC gadgets
- Pairing code
- Groth16 verifier logic
- Real Halo2 cryptographic circuits
- Proof generation or verification
- Performance tuning of cryptographic paths

Do not implement stage 1 during this phase.

## Repository Map

- `crates/wrapper-core`: domain models, traits, config, errors, metadata, serialization-friendly structs
- `crates/wrapper-circuits`: Halo2-facing shells and future circuit module boundaries
- `crates/wrapper-backends`: backend adapters, artifact parsing entry points, future ecosystem integrations
- `crates/wrapper-cli`: developer commands and diagnostics
- `crates/wrapper-tests`: shared fixtures, helpers, and end-to-end test harness
- `docs/architecture.md`: intended layering and data flow
- `docs/roadmap.md`: staged implementation plan
- `docs/decisions/0001-initial-workspace-structure.md`: ADR for the workspace split

## Crate Responsibilities

`wrapper-core`

- Must remain mostly domain-oriented.
- Prefer no Halo2 dependency unless a boundary cannot be expressed otherwise.
- Own shared enums, traits, config structs, metadata, and stable public concepts.

`wrapper-circuits`

- Own Halo2-facing circuit shells, future chip modules, layout planning, and circuit-specific configuration.
- Should depend on `wrapper-core`.
- Must not absorb artifact parsing or backend-specific concerns.

`wrapper-backends`

- Own parsing, loaders, serialization adapters, and future ecosystem bridges.
- Should depend on `wrapper-core`.
- Must not define core architecture or circuit semantics independently.

`wrapper-cli`

- Own user-facing commands, output formatting, and developer diagnostics.
- Must report missing functionality honestly.

`wrapper-tests`

- Own fixtures, shared test helpers, and future end-to-end harness organization.
- Should not become a dumping ground for business logic.

## Rules for Architectural Changes

- Preserve the separation between domain, circuits, and backends unless there is a documented reason to merge boundaries.
- Update `docs/architecture.md` and the relevant ADR when changing public architecture.
- Prefer adding narrow interfaces in `wrapper-core` over leaking backend or circuit implementation details across crates.
- Avoid speculative abstractions not justified by an immediate design need.

## Rules for Adding Dependencies

- Add dependencies conservatively.
- Prefer workspace-managed dependency versions.
- Do not add heavy cryptography crates unless they are required by the current stage and compile in CI.
- Document why a new dependency belongs now, not later.
- If a dependency is stage-specific or optional, gate it behind a clearly named feature when appropriate.

## Rules for Implementing Future Cryptographic Code

- Start from documented interfaces and staged roadmap items.
- Add tests and docs alongside any cryptographic implementation.
- Keep arithmetic, ECC, pairing, and verifier logic explicit and reviewable.
- Do not hide critical cryptographic behavior behind generic abstractions that obscure invariants.
- Record major cryptographic architecture choices in `docs/decisions/`.

## Coding Standards

- Prefer explicit, readable Rust over cleverness.
- Use crate-level docs and module docs when they clarify purpose.
- Keep placeholder code small but realistic.
- Avoid fake implementations that imply correctness.
- Keep comments purposeful and sparse.

## Error Handling Standards

- Use `thiserror` for library-facing error types.
- Use `anyhow` at CLI boundaries or orchestration boundaries where context aggregation is helpful.
- Errors should state what failed, at what boundary, and whether the feature is intentionally unimplemented.

## Testing Standards

- Every new public behavior should have at least one test at the appropriate layer.
- Keep unit tests near the crate that owns the behavior.
- Use `wrapper-tests` for cross-crate fixtures and integration coverage.
- Do not add cryptographic “smoke tests” that only simulate correctness without real implementation.

## Documentation Standards

- Update the README when the contributor workflow or implemented scope changes.
- Update `docs/roadmap.md` when stage boundaries or sequencing change.
- Add or amend ADRs for architectural decisions that affect crate ownership or public interfaces.
- Be explicit about what is scaffolded versus implemented.

## How to Propose a Change

1. Identify the stage and boundary the change belongs to.
2. Check whether the change fits an existing crate responsibility.
3. Update docs first when the architecture is affected.
4. Implement the smallest honest increment.
5. Add tests that prove the increment, not future claims.

## What Not To Do

- Do not collapse crates for convenience.
- Do not place Halo2-specific concerns in `wrapper-core` without strong justification.
- Do not add pairings, ECC, foreign field arithmetic, or Groth16 verifier logic during initialization.
- Do not write placeholder code that pretends proofs are verified.
- Do not add backend integrations that are not exercised by the current build.

## Explicit Warning for This Initialization Task

For this initialization task specifically, do not implement:

- foreign field arithmetic
- ECC gadgets
- pairing gadgets or arithmetic
- Groth16 BN254 verification logic
- real Halo2 cryptographic circuits

Only build the skeleton, workflow, docs, and honest interfaces.

## Preferred Incremental Workflow for Future Stages

1. Extend `wrapper-core` with the minimal new domain concept.
2. Introduce the required backend or circuit boundary in the owning crate.
3. Expose a small diagnostic or validation path in the CLI if useful.
4. Add fixture-driven tests.
5. Document the design decision before scaling implementation breadth.

