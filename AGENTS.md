# AGENTS.md

## Project Purpose

This repository is a Rust workspace for a staged research and engineering effort around a Halo2-based wrapper that may eventually verify Groth16 BN254 proofs inside an outer Halo2 proof system.

The project is intentionally incremental. The current codebase now includes a first circuit-backed Week 1 foundation for BN254 foreign-field arithmetic and minimal G1 work, but it is still far from a Groth16 wrapper verifier.

## Current Phase and Scope Boundaries

Current phase: Week 1 foundation plus sanity checks.

Implemented in scope today:

- Workspace structure, crate boundaries, docs, CLI, CI, and benchmark conventions
- BN254 foreign-field support in `wrapper-circuits` backed by `midnight-circuits` / `midnight-proofs`
- Circuit-backed `fp add`, `fp mul`, and related minimal field wiring
- Minimal BN254 G1 support backed by Midnight foreign ECC chips
- Circuit-backed G1 addition
- Coordinate-to-point construction with on-curve enforcement
- Real layout and row visibility through the Halo2/Midnight cost model
- Deterministic randomized tests against arkworks
- Criterion sanity benchmarks for the currently implemented Week 1 circuits

Out of scope right now:

- Fp2 / Fp12
- G2
- pairings
- Miller loop
- final exponentiation
- Groth16 verifier logic
- MSM as a public supported layer
- wrapper verifier circuit composition
- production optimization of layout/cost beyond the narrow Week 1 circuits

Do not treat the current code as a full verifier foundation. It is a deliberately narrow primitive layer.

## Repository Map

- `crates/wrapper-core`: domain models, traits, config, errors, metadata, capability/status reporting
- `crates/wrapper-circuits`: Halo2-facing code, Midnight-backed BN254 field/G1 layer, planning, layout reporting
- `crates/wrapper-backends`: backend adapters, artifact parsing entry points, future ecosystem integrations
- `crates/wrapper-cli`: developer commands and diagnostics
- `crates/wrapper-tests`: shared fixtures, benchmark entry points, and integration helpers
- `docs/architecture.md`: intended layering and current Week 1 circuit boundaries
- `docs/roadmap.md`: staged implementation plan
- `docs/benchmarking.md`: benchmark structure and conventions
- `docs/decisions/0001-initial-workspace-structure.md`: ADR for the workspace split

## Crate Responsibilities

`wrapper-core`

- Must remain mostly domain-oriented.
- Prefer no Halo2 dependency unless a boundary cannot be expressed otherwise.
- Own shared enums, traits, config structs, metadata, capabilities, and stable public concepts.
- Must not absorb chip-specific or region-specific logic.

`wrapper-circuits`

- Own Halo2-facing code, Midnight integration, circuit planning, and primitive gadget boundaries.
- Currently owns the BN254 `AssignedFp` and `AssignedG1` circuit-backed layer.
- Should depend on `wrapper-core`.
- Must not absorb artifact parsing or backend-specific concerns.

`wrapper-backends`

- Own parsing, loaders, serialization adapters, and future ecosystem bridges.
- Should depend on `wrapper-core`.
- Must not define circuit semantics independently of `wrapper-circuits`.

`wrapper-cli`

- Own user-facing commands, output formatting, and developer diagnostics.
- Must report missing functionality honestly.
- Should expose measured Week 1 status without overstating what is implemented.

`wrapper-tests`

- Own fixtures, shared test helpers, and benchmark entry points.
- Should host cross-crate integration coverage and Criterion runners.
- Should not become a dumping ground for reusable circuit logic that belongs in `wrapper-circuits`.

## Rules for Architectural Changes

- Preserve the separation between domain, circuits, and backends unless there is a documented reason to change it.
- Update `docs/architecture.md` and the relevant ADR when changing public architecture or ownership boundaries.
- Prefer narrow interfaces in `wrapper-core` over leaking circuit implementation details across crates.
- Keep the current Week 1 circuit-backed layer small and explicit.
- Do not introduce speculative abstractions for later pairing/verifier work unless they are needed immediately.

## Rules for Adding Dependencies

- Add dependencies conservatively.
- Prefer workspace-managed dependency versions.
- Heavy cryptographic dependencies must earn their place through current-stage use, tests, and CI viability.
- Document why a new dependency belongs now, not later.
- Feature-gate stage-specific dependencies when appropriate.
- For current circuit work, prefer existing Midnight and arkworks infrastructure over inventing parallel stacks.

## Rules for Implementing Cryptographic Code

- Start from documented interfaces and roadmap items.
- Add tests and docs alongside any cryptographic implementation.
- Keep arithmetic, ECC, pairing, and verifier logic explicit and reviewable.
- Do not hide critical behavior behind abstractions that obscure invariants or cost.
- Record major cryptographic architecture choices in `docs/decisions/`.
- Prefer extending the existing Midnight-backed BN254 layer over creating a second incompatible primitive stack.

## Specific Week 1 Guidance

The current Week 1 layer is built around:

- `midnight-circuits`
- `midnight-proofs`
- `midnight-curves`
- arkworks as the reference implementation in tests

When touching Week 1 code:

- keep `fp` work limited to the currently supported primitive surface unless the roadmap explicitly expands it
- keep G1 work limited to the currently supported primitive surface unless the roadmap explicitly expands it
- preserve real layout measurement support
- keep benchmarks honest and tied to actually implemented circuits
- keep CLI reporting aligned with the measured state of the codebase

## Coding Standards

- Prefer explicit, readable Rust over cleverness.
- Use crate-level docs and module docs when they clarify purpose.
- Keep comments purposeful and sparse.
- Keep circuit-backed adapters thin where possible.
- Avoid duplicate primitive stacks or parallel APIs for the same concept.
- Do not leave misleading stubs that imply verifier completeness.

## Error Handling Standards

- Use `thiserror` for library-facing error types.
- Use `anyhow` at CLI or orchestration boundaries where context aggregation is helpful.
- Errors should state what failed, at what boundary, and whether the feature is intentionally unimplemented.
- If a circuit path is deliberately unsupported at this stage, say so explicitly instead of faking behavior.

## Testing Standards

- Every new public behavior should have at least one test at the owning layer.
- Keep unit tests near the crate that owns the behavior.
- Use arkworks as the reference implementation for BN254 field and G1 sanity checks when appropriate.
- Keep randomized tests deterministic via fixed seeds unless there is a strong reason not to.
- Use `wrapper-tests` for shared fixtures, integration coverage, and benchmark entry points.
- Do not add tests that imply pairing or verifier support before those stages exist.

## Benchmarking Standards

- Use Criterion.
- Keep benchmark names in the `bench_<module>_<operation>` form.
- Benchmarks must reflect real implemented circuits, not aspirational future behavior.
- Do not make performance claims beyond what the current benchmark actually measures.
- When changing benchmark structure, update `docs/benchmarking.md` and `wrapper-cli bench-info`.

## Documentation Standards

- Update the README when implemented scope or contributor workflow changes.
- Update `docs/architecture.md` when circuit boundaries or ownership changes.
- Update `docs/roadmap.md` when stage boundaries or sequencing change.
- Add or amend ADRs for architectural decisions that affect crate ownership or public interfaces.
- Be explicit about what is circuit-backed, what is reference-tested, and what is still missing.

## How to Propose a Change

1. Identify the stage and boundary the change belongs to.
2. Check whether the change fits an existing crate responsibility.
3. Update docs first when the architecture or scope changes.
4. Implement the smallest honest increment.
5. Add tests that prove the increment, not future claims.
6. Verify `cargo check`, `cargo test`, and relevant benches or CLI paths when applicable.

## What Not To Do

- Do not collapse crates for convenience.
- Do not place Halo2-specific concerns in `wrapper-core` without strong justification.
- Do not implement Fp2, G2, pairings, Miller loop, final exponentiation, or Groth16 verifier logic unless the task explicitly asks for that stage.
- Do not write placeholder code that pretends proofs are verified.
- Do not add a second BN254 primitive implementation path that competes with the Midnight-backed one without a documented reason.
- Do not overclaim performance or soundness from the current Week 1 circuits.

## Explicit Warning for Current Tasks

For tasks in the current repository state, do not assume that because `fp add`, `fp mul`, and minimal G1 addition exist, the project is ready for:

- pairing gadgets
- Groth16 verification
- wrapped verifier composition
- public-input verifier logic
- G2 arithmetic
- full MSM infrastructure

Those remain future-stage work unless the task explicitly advances the roadmap.

## Preferred Incremental Workflow for Next Stages

1. Extend `wrapper-core` only with the minimal new domain concept.
2. Expand `wrapper-circuits` in the narrowest possible way around the existing Midnight-backed foundation.
3. Preserve or improve real layout visibility when adding new circuit-backed primitives.
4. Add arkworks-backed or equivalent reference tests.
5. Add or update benchmarks only for code that truly exists.
6. Document the design decision before scaling implementation breadth.
