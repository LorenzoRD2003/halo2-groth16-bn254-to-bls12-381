# AGENTS.md

## Project Purpose

This repository is a Rust workspace for a staged research and engineering effort around a Halo2-based wrapper that may eventually verify Groth16 BN254 proofs inside an outer Halo2 proof system.

The project is intentionally incremental. The current codebase now includes a circuit-backed BN254 primitive layer covering Week 1 foundations, the narrow Week 2 slices, and the current Week 3 extension-field slice, but it is still far from a Groth16 wrapper verifier.

## Current Phase and Scope Boundaries

Current phase: Stage 1 / Week 3 narrow primitive expansion.

Implemented in scope today:

- Workspace structure, crate boundaries, docs, CLI, CI, and benchmark conventions
- BN254 foreign-field support in `wrapper-circuits` backed by `midnight-circuits` / `midnight-proofs`
- Circuit-backed `fp add`, `fp mul`, and related minimal field wiring
- Circuit-backed BN254 `fp2` support represented as `a + bu` with `u^2 = -1`
- `AssignedFp2` over two `AssignedFp` coordinates with `new`, assignment, `zero`, `one`, `add`, `sub`, `neg`, `mul`, `square`, and equality helpers
- Circuit-backed BN254 `fp6` support represented as `c0 + c1 * v + c2 * v^2`
- `AssignedFp6` over three `AssignedFp2` coordinates with `new`, assignment, `zero`, `one`, `add`, `sub`, `neg`, `mul`, `square`, and equality helpers
- Circuit-backed BN254 `fp12` support represented as `c0 + c1 * w`
- `AssignedFp12` over two `AssignedFp6` coordinates with `new`, assignment, `zero`, `one`, `add`, `sub`, `neg`, `mul`, `square`, and equality helpers
- Minimal BN254 G1 support backed by Midnight foreign ECC chips
- Circuit-backed G1 addition
- Coordinate-to-point construction with on-curve enforcement
- Minimal BN254 G2 affine support backed by `AssignedFp2`
- `AssignedG2Affine` with assignment, `neg`, `assert_equal`, and explicit twist `assert_on_curve`
- Narrow BN254 G2 projective support in Jacobian coordinates over `AssignedFp2`
- `AssignedG2Projective` with reserved identity encoding plus `from_affine`, `neg`, `double`, and incomplete `add`
- Real layout and row visibility through the Halo2/Midnight cost model
- Deterministic arkworks-backed tests for `Fp`, `Fp2`, `Fp6`, `Fp12`, G1, and the current narrow G2 affine/projective behavior
- Criterion sanity benchmarks for the currently implemented primitive circuits
- a single authoritative BN254 primitive path in `wrapper-circuits/src/bn254/`

Out of scope right now:

- G2 subgroup checks
- scalar multiplication on G2
- pairings
- line functions
- Miller loop
- final exponentiation
- Groth16 verifier logic
- MSM as a public supported layer
- wrapper verifier circuit composition
- production optimization of layout/cost beyond the narrow implemented sanity circuits

Do not treat the current code as a full verifier foundation. It is a deliberately narrow primitive layer.

## Repository Map

- `crates/wrapper-core`: domain models, traits, config, errors, metadata, capability/status reporting
- `crates/wrapper-circuits`: Halo2-facing code, Midnight-backed BN254 primitive layer, planning, layout reporting
- `crates/wrapper-backends`: backend adapter placeholders, artifact parsing entry points, future ecosystem integrations
- `crates/wrapper-cli`: developer commands and diagnostics
- `crates/wrapper-tests`: shared fixtures, benchmark entry points, and integration helpers
- `docs/architecture.md`: intended layering and current primitive boundaries
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
- Currently owns the BN254 `AssignedFp`, `AssignedFp2`, `AssignedFp6`, `AssignedFp12`, `AssignedG1`, `AssignedG2Affine`, and narrow `AssignedG2Projective` circuit-backed layer.
- Keeps the active BN254 primitive implementation under `src/bn254/`, split by concern instead of one monolithic file.
- Should depend on `wrapper-core`.
- Must not absorb artifact parsing or backend-specific concerns.
- Keep dead compatibility shims and obsolete host-side leftovers out of the crate.

`wrapper-backends`

- Own parsing, loaders, serialization adapters, and future ecosystem bridges.
- Should depend on `wrapper-core`.
- Must not define circuit semantics independently of `wrapper-circuits`.
- It is still mostly placeholder territory in the current repo state.

`wrapper-cli`

- Own user-facing commands, output formatting, and developer diagnostics.
- Must report missing functionality honestly.
- Should expose measured primitive status without overstating what is implemented.

`wrapper-tests`

- Own fixtures, shared test helpers, and benchmark entry points.
- Should host cross-crate integration coverage and Criterion runners.
- Should not become a dumping ground for reusable circuit logic that belongs in `wrapper-circuits`.

## Rules for Architectural Changes

- Preserve the separation between domain, circuits, and backends unless there is a documented reason to change it.
- Update `docs/architecture.md` and the relevant ADR when changing public architecture or ownership boundaries.
- Prefer narrow interfaces in `wrapper-core` over leaking circuit implementation details across crates.
- Keep the current primitive layer small and explicit.
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

## Specific Current Primitive Guidance

The current primitive layer is built around:

- `midnight-circuits`
- `midnight-proofs`
- `midnight-curves`
- arkworks as the reference implementation in tests

When touching the current BN254 primitive code:

- keep `fp` work limited to the currently supported primitive surface unless the roadmap explicitly expands it
- keep `fp2` work aligned with the current representation `Fq2(c0, c1)` and `u^2 = -1`
- keep `fp6` work aligned with the current representation `Fq6(c0, c1, c2)` and `v^3 = 9 + u`
- keep `fp12` work aligned with the current representation `Fq12(c0, c1)` and `w^2 = v`
- keep G1 work limited to the currently supported primitive surface unless the roadmap explicitly expands it
- keep G2 work limited to the currently supported affine plus narrow Jacobian projective surface unless the roadmap explicitly expands it
- preserve real layout measurement support
- keep benchmarks honest and tied to actually implemented circuits
- keep CLI reporting aligned with the measured state of the codebase

Concrete BN254 conventions already in use:

- `AssignedFp2` follows the standard BN254 extension representation `a + bu`
- `Fq2` coordinate order is `(c0, c1)` to match arkworks
- `u^2 = -1`
- `AssignedFp6` follows `c0 + c1 * v + c2 * v^2`
- `Fq6` coordinate order is `(c0, c1, c2)` to match arkworks
- the cubic nonresidue is `9 + u`, so `Fp6 = Fp2[v] / (v^3 - (9 + u))`
- `AssignedFp12` follows `c0 + c1 * w`
- `Fq12` coordinate order is `(c0, c1)` to match arkworks
- the quadratic nonresidue is `v = Fp6(0, 1, 0)`, so `Fp12 = Fp6[w] / (w^2 - v)`
- minimal G2 affine on-curve checks use the arkworks BN254 twist equation `y^2 = x^3 + b`
- the twist coefficient is `b = 3 / (u + 9)` with the exact arkworks value
  `Fq2(19485874751759354771024239261021720505790618469301721065564631296452457478373, 266929791119991161246907387137283842545076965332900288569378510910307636690)`

Current measured primitive costs from `wrapper-cli doctor`:

- `fp add`: 40 rows / 58 queries, `k=9`
- `fp mul`: 38 rows / 58 queries, `k=9`
- `fp2 add`: 80 rows / 58 queries, `k=9`
- `fp2 mul`: 152 rows / 58 queries, `k=9`
- `fp2 square`: 114 rows / 58 queries, `k=9`
- `fp6 add`: 240 rows / 58 queries, `k=9`
- `fp6 mul`: 1384 rows / 58 queries, `k=11`
- `fp6 square`: 868 rows / 58 queries, `k=10`
- `fp12 add`: 480 rows / 58 queries, `k=9`
- `fp12 mul`: 4538 rows / 58 queries, `k=13`
- `fp12 square`: 3056 rows / 58 queries, `k=12`
- `g1 add`: 319 rows / 105 queries, `k=9`
- `g2 on_curve`: 400 rows / 58 queries, `k=9`
- `g2 neg`: 930 rows / 58 queries, `k=10`
- `g2 proj from_affine`: 970 rows / 58 queries, `k=10`
- `g2 proj double`: 2594 rows / 58 queries, `k=12`
- `g2 proj add`: 4582 rows / 58 queries, `k=13`

Interpretation guidance:

- `g2 neg` is not a measure of a raw sign flip alone; the current benchmark circuit includes assignment, on-curve checks, negation, and equality against the expected output
- `fp12 mul` and `fp12 square` are measurements of the actual sanity circuits over the implemented tower, not optimized pairing-ready kernels
- cost numbers should always be described as measurements of the actual sanity circuits, not abstract algebraic lower bounds

## Coding Standards

- Prefer explicit, readable Rust over cleverness.
- Use crate-level docs and module docs when they clarify purpose.
- Keep comments purposeful and sparse.
- Keep circuit-backed adapters thin where possible.
- Avoid duplicate primitive stacks or parallel APIs for the same concept.
- Prefer removing obsolete compatibility files once the Midnight-backed path replaces them.
- Do not leave misleading stubs that imply verifier completeness.
- Delete files that have become genuinely unused instead of keeping stale alternative paths around.

## Error Handling Standards

- Use `thiserror` for library-facing error types.
- Use `anyhow` at CLI or orchestration boundaries where context aggregation is helpful.
- Errors should state what failed, at what boundary, and whether the feature is intentionally unimplemented.
- If a circuit path is deliberately unsupported at this stage, say so explicitly instead of faking behavior.
- Do not keep custom error layers that are no longer used after an integration shift.

## Testing Standards

- Every new public behavior should have at least one test at the owning layer.
- Keep unit tests near the crate that owns the behavior.
- Use arkworks as the reference implementation for BN254 field, Fp2, G1, and minimal G2 affine sanity checks when appropriate.
- Keep randomized tests deterministic via fixed seeds unless there is a strong reason not to.
- Use `wrapper-tests` for shared fixtures, integration coverage, and benchmark entry points.
- Do not add tests that imply pairing or verifier support before those stages exist.

Current test expectations for the primitive layer:

- `Fp2` tests should include algebra identities, deterministic randomized add/mul/square checks, and edge-oriented real/imaginary cases
- `Fp6` tests should include algebra identities, deterministic randomized add/mul/square checks, and structured single-coordinate cases
- `Fp12` tests should include algebra identities, deterministic randomized add/mul/square checks, and structured `c0`-only / `c1`-only cases
- minimal G2 tests should include valid affine points, negative on-curve cases, negation validity, and equality behavior
- narrow G2 projective tests should stay explicit about the supported domain: `from_affine`, `neg`, `double`, incomplete `add`, and reserved identity encoding

## Benchmarking Standards

- Use Criterion.
- Keep benchmark names in the `bench_<module>_<operation>` form.
- Benchmarks must reflect real implemented circuits, not aspirational future behavior.
- Do not make performance claims beyond what the current benchmark actually measures.
- When changing benchmark structure, update `docs/benchmarking.md` and `wrapper-cli bench-info`.

Current benchmark entry points include:

- `bench_fp_add`
- `bench_fp_mul`
- `bench_fp2_add`
- `bench_fp2_mul`
- `bench_fp2_square`
- `bench_fp6_add`
- `bench_fp6_mul`
- `bench_fp6_square`
- `bench_fp12_add`
- `bench_fp12_mul`
- `bench_fp12_square`
- `bench_g1_add`
- `bench_g2_on_curve`
- `bench_g2_neg`
- `bench_g2_proj_from_affine`
- `bench_g2_proj_double`
- `bench_g2_proj_add`

## Documentation Standards

- Update the README when implemented scope or contributor workflow changes.
- Update `docs/architecture.md` when circuit boundaries or ownership changes.
- Update `docs/roadmap.md` when stage boundaries or sequencing change.
- Add or amend ADRs for architectural decisions that affect crate ownership or public interfaces.
- Be explicit about what is circuit-backed, what is reference-tested, and what is still missing.
- When cleanup removes obsolete files or paths, reflect the new simpler state in contributor docs.

When refactoring `wrapper-circuits/src/bn254/`:

- keep the public API stable through `bn254/mod.rs` re-exports when possible
- prefer splitting by concept, for example `types.rs`, `field.rs`, `fp2.rs`, `g2.rs`, `metrics.rs`, `tests.rs`
- if docs mention the primitive path, keep them pointed at `src/bn254/`, not the old deleted `src/bn254.rs`

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
- Do not implement pairings, Miller loop, final exponentiation, or Groth16 verifier logic unless the task explicitly asks for that stage.
- Do not jump from minimal G2 affine support to G2 arithmetic or subgroup logic unless the task explicitly asks for it.
- Do not write placeholder code that pretends proofs are verified.
- Do not add a second BN254 primitive implementation path that competes with the Midnight-backed one without a documented reason.
- Do not overclaim performance or soundness from the current sanity circuits.

## Explicit Warning for Current Tasks

For tasks in the current repository state, do not assume that because `fp add`, `fp mul`, `fp2`, minimal G1, and minimal G2 affine support exist, the project is ready for:

- generalized `Fp12` helper optimizations for pairing workloads
- line-function gadgets
- pairing gadgets
- Groth16 verification
- wrapped verifier composition
- public-input verifier logic
- G2 arithmetic beyond the currently implemented narrow Jacobian `from_affine` / `neg` / `double` / incomplete `add` slice
- full MSM infrastructure

Those remain future-stage work unless the task explicitly advances the roadmap.

## Preferred Incremental Workflow for Next Stages

1. Extend `wrapper-core` only with the minimal new domain concept.
2. Expand `wrapper-circuits` in the narrowest possible way around the existing Midnight-backed foundation.
3. Preserve or improve real layout visibility when adding new circuit-backed primitives.
4. Add arkworks-backed or equivalent reference tests.
5. Add or update benchmarks only for code that truly exists.
6. Document the design decision before scaling implementation breadth.
