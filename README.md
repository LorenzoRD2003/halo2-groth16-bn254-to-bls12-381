# Halo2 Wrapper Workspace

This repository is a Rust workspace for a staged research and engineering effort around a Halo2-based outer proof system that wraps Groth16 BN254 proofs inside a canonical Halo2/Midnight outer circuit.

The current phase is still intentionally narrow, but it is no longer just repository bootstrap. The repo now includes the BN254 primitive layer, the narrow Groth16 BN254 verifier slice, the canonical `OuterWrapperCircuit`, and a real direct Halo2/Midnight backend lane that can `setup -> prove -> verify` honest outer artifacts. It still stops well short of a broad or production-ready wrapper system.

## Current Status

What the repository currently contains:

- A multi-crate Rust workspace with explicit boundaries between domain logic, circuit-facing code, backend adapters, CLI tooling, and test harness code.
- Week 1 BN254 foundations in `wrapper-circuits`, organized under `src/bn254/`: Midnight-backed foreign-field and G1 circuits with real layout measurements.
- A first Week 2 BN254 Fp2 layer in `wrapper-circuits`, also organized under `src/bn254/`, built from two circuit-backed `AssignedFp` coordinates.
- A minimal BN254 Fp6 layer in `wrapper-circuits`, built as three circuit-backed `AssignedFp2` coordinates using the arkworks BN254 tower.
- A minimal BN254 Fp12 layer in `wrapper-circuits`, built as two circuit-backed `AssignedFp6` coordinates using the same arkworks / Midnight BN254 tower.
- A minimal Week 2 BN254 G2 affine layer in `wrapper-circuits`, backed by `AssignedFp2` coordinates with circuit-backed negation and twist on-curve validation.
- A narrow Week 2 BN254 G2 projective layer in Jacobian coordinates `(X:Y:Z)` with affine embedding, negation, doubling, incomplete addition, measured costs, and arkworks-backed sanity tests.
- A Week 3 BN254 Miller-path G2 step layer with a dedicated homogeneous-projective state, `double_with_line`, `mixed_add_with_line`, and Miller-ready sparse line coefficients.
- A real direct outer backend lane in `wrapper-backends/src/outer.rs` that runs setup, proof generation, and verification over the canonical `OuterWrapperCircuit`.
- A narrow Week 4 pairing core: real Miller loop, final exponentiation, and verifier-shaped pairing check.
- A first Week 5 Groth16 BN254 verifier slice with real `snarkjs` proof/VK/public-input parsing, verifier-only `vk_x` accumulation, and one end-to-end pairing-product-check path.
- Generic `snarkjs` Groth16 BN254 artifact-set loading in `wrapper-backends`, including named public-input views when the caller supplies semantic names.
- A real Halo2/Midnight outer wrapper circuit in `wrapper-circuits` that reuses the landed narrow Groth16 BN254 verifier slice and exposes the frozen outer statement as public inputs.
- Domain-level wrapper planning and execution-package modeling in `wrapper-core`, including `WrapperJob`, `WrapperExecutionPackage`, honest outer artifact shapes, and execution-result modeling.
- The direct outer artifact model is now honest to the actual backend: `halo2-plonkish` / `bn254`, serialized with `serde` JSON carrying hex-encoded `SerdeFormat::Processed` payloads for proofs, PLONK verification keys, and KZG verifier params.
- A canonical R1CS line now exists under `crates/wrapper-circuits/src/r1cs/`, including deterministic lowering, identity hashing, zkInterface-style export, and a first Arkworks adapter, but it should currently be treated as an alternate / future backend lane rather than the critical path for the real outer wrapper flow.
- A real Semaphore Groth16 BN254 fixture under `crates/wrapper-tests/fixtures/groth16/semaphore/` used to validate the direct outer lane through a real end-to-end integration test.
- Contributor-oriented documentation covering architecture, roadmap, and initial design decisions.
- A `wrapper-cli` binary with honest developer commands for environment inspection, configuration validation, primitive reporting, narrow layout profiling, and real direct outer execution.

What is explicitly not implemented yet:

- Broad public pairing gadgets or generalized pairing APIs
- Generalized Groth16 verifier frameworks beyond the first narrow BN254 slice
- G2 subgroup checks or scalar multiplication
- Broad backend adapters beyond the current narrow `snarkjs` BN254 parser path
- Fast always-on CI for the expensive direct outer `setup/prove/verify` lane
- Cryptographic soundness claims of any kind

This repository now includes the primitive BN254 foundation plus the first narrow Groth16 BN254 verifier slice, but it is still far from a broad or production-ready wrapper verifier.

## Quick Context Routes

Use the shortest route that matches the task:

- Current repo snapshot: `README.md` -> `AGENTS.md` -> `docs/architecture.md`
- Groth16 verifier slice: `crates/wrapper-circuits/src/groth16.rs` -> `crates/wrapper-backends/src/snarkjs.rs` -> `crates/wrapper-tests/fixtures/groth16/circom_multiplier2/README.md`
- Wrapper planning / package flow: `crates/wrapper-backends/src/groth16.rs` -> `crates/wrapper-core/src/job.rs` -> `crates/wrapper-core/src/package.rs` -> `crates/wrapper-core/src/output.rs` -> `crates/wrapper-core/src/execution.rs`
- Semaphore migration fixture: `crates/wrapper-tests/fixtures/groth16/semaphore/README.md` -> `crates/wrapper-tests/src/lib.rs` -> `crates/wrapper-cli/src/main.rs`
- ZK Email integration study: `docs/plans/0004-zk-email-integration-plan.md` -> `crates/wrapper-tests/fixtures/groth16/semaphore/README.md` -> `docs/plans/0003-plutus-aiken-integration-plan.md`
- Real `.circom` integration plan: `docs/real-circom-wrapper-integration-plan.md`
- Canonical R1CS backend status: `docs/r1cs-backend-status.md`
- Outer prover strategy: `docs/outer-prover-strategy-plan.md`
- Direct setup-cost reduction: `docs/decisions/0003-direct-outer-setup-cost-reduction.md`
- Ultra-fine finalize profiling plan: `docs/plans/0006-finalize-checkpoint-profiling-plan.md`
- H poly speed follow-up work after memory is solved: `docs/h-poly-followup-speed-plan.md`
- Pairing / final exponentiation: `crates/wrapper-circuits/src/bn254/g2/miller.rs` -> `crates/wrapper-circuits/src/bn254/host/pairing_host.rs` -> `docs/midnight-optimizations.md`
- Layout profiling / optimization: `crates/wrapper-circuits/src/groth16/profiling.rs` -> `crates/wrapper-cli/src/main.rs` -> `docs/profiling.md`
- Midnight-local optimization opportunities: `docs/midnight-optimizations.md` -> `crates/wrapper-circuits/src/bn254/types.rs` -> `crates/wrapper-circuits/src/bn254/fp6.rs`
- Scope / stage boundaries: `AGENTS.md` `Current Phase and Scope Boundaries` -> `docs/roadmap.md`

Top-level doc roles:

- `README.md`: quickest orientation and common commands
- `AGENTS.md`: binding contributor rules plus the most detailed context-loading map
- `docs/architecture.md`: ownership boundaries and current implementation shape
- `docs/roadmap.md`: stage intent and explicit non-goals
- `docs/profiling.md`: how to measure layout-cost changes
- `docs/midnight-optimizations.md`: prioritized Midnight primitives and local optimization candidates for repeated BN254 tower operations
- `docs/plans/0002-cyclotomic-unitary-kernel-design.md`: proposed compressed-torus region design for repeated `cyclotomic * unitary_inverse(cyclotomic)` work in the hard part
- `docs/decisions/0003-direct-outer-setup-cost-reduction.md`: accepted direction for reducing direct outer setup cost via a lean setup artifact and later params caching
- `docs/decisions/0004-local-midnight-proofs-patch.md`: accepted rationale for carrying a local `midnight-proofs` patch to support richer direct setup/prove artifacts
- `docs/plans/0006-finalize-checkpoint-profiling-plan.md`: implementation plan for ultra-fine `prove-finalize` checkpoint logging, iteration heartbeats, memory snapshots, elapsed-time profiling, and real-time log inspection
- `docs/h-poly-followup-speed-plan.md`: deferred speed follow-ups for the retained chunked `h_poly` path after the current memory blocker is solved
- `docs/plans/0004-zk-email-integration-plan.md`: phased plan for the first larger Circom-origin integration track using ZK Email as the reference case
- `docs/real-circom-wrapper-integration-plan.md`: implementation plan to finish the real `.circom` -> outer-wrapper end-to-end path
- `docs/r1cs-backend-status.md`: current state of the canonical R1CS line and why it is currently an alternate backend / later phase
- `docs/outer-prover-strategy-plan.md`: current proving-strategy decision for the canonical outer circuit and the direct backend surface

Current direct execution note:

- the repository now exposes split direct commands:
  - `execute-wrapper-direct-setup`
  - `execute-wrapper-direct-prove`
  - `execute-wrapper-direct-prove-trace`
  - `execute-wrapper-direct-prove-finalize`
  - `execute-wrapper-direct-verify`
- the repository now uses a richer direct setup artifact plus a local `midnight-proofs` patch so the direct prove path avoids rerunning `keygen_pk(...)`
- the current next suspected memory hotspot is eager coset materialization in the patched prover; see `docs/decisions/0003-direct-outer-setup-cost-reduction.md` and `docs/decisions/0004-local-midnight-proofs-patch.md`
- the split `prove-trace` / `prove-finalize` flow exists so the pre-`compute_h_poly(...)` phase can be cached and rerun independently from the memory-heavy finalization stage
- current caveat: `prove-trace` is now working, but `prove-finalize` is still the active memory-reduction target
- `docs/decisions/0002-bn254-local-optimization-policy.md`: retained and rejected local BN254 pairing-core optimization directions

## Planned Architecture

The intended shape of the project is:

- `wrapper-core`: domain-oriented types, traits, config, errors, metadata, and public architectural contracts
- `wrapper-core`: also owns wrapper-job planning, execution-package modeling, expected output-artifact shapes, and execution-result modeling
- `wrapper-circuits`: Halo2-facing circuits, current Midnight-backed BN254 primitive layer, layout descriptions, and future gadget integration points
- `wrapper-backends`: artifact loading, parser adapters, proof/VK material ingestion, generic Groth16 BN254 bundles, and future external backend bridges
- `wrapper-cli`: developer-facing commands for validation, inspection, and future orchestration
- `wrapper-tests`: shared fixtures, helpers, and future end-to-end integration coverage

The design keeps `wrapper-core` mostly independent from Halo2 so project concepts can stay stable even as proof-system integration evolves.

## Workspace Map

```text
.
├── Cargo.toml
├── README.md
├── AGENTS.md
├── docs/
│   ├── architecture.md
│   ├── benchmarking.md
│   ├── cyclotomic-unitary-kernel-design.md
│   ├── midnight-optimizations.md
│   ├── outer-prover-strategy-plan.md
│   ├── profiling.md
│   ├── real-circom-wrapper-integration-plan.md
│   ├── roadmap.md
│   ├── plans/
│   └── decisions/
│       ├── 0001-initial-workspace-structure.md
│       └── 0002-bn254-local-optimization-policy.md
└── crates/
    ├── wrapper-core/
    ├── wrapper-circuits/
    ├── wrapper-backends/
    ├── wrapper-cli/
    └── wrapper-tests/
```

## Where To Read Next

- Start in `README.md` when you need the fastest high-level reload.
- Go to `AGENTS.md` before editing code or docs so you inherit repo-specific constraints.
- Go to `docs/architecture.md` when deciding where code should live.
- Go to `docs/roadmap.md` when checking whether an idea belongs in the current stage.
- Go to `docs/profiling.md` and `docs/midnight-optimizations.md` for optimization work.
- Go to `docs/midnight-optimizations.md` when you want the current prioritized list of local Midnight-backed optimization opportunities.
- Go to `docs/midnight-optimizations.md` when you want local tower wins driven by existing `midnight-circuits` primitives such as `mul_by_constant` and `add_constant`, or when you need the current record of which `linear_combination` rewrites were already measured and ruled out and which `add_constant` uses actually paid off.
- Go to `docs/midnight-optimizations.md` when working on `exp_by_neg_x(...)`; it now records the retained signed-window chain that improved the final-exponentiation hard part.
- Go to `docs/midnight-optimizations.md` when working on compressed cyclotomic squaring; it now records the retained compressed-square rewrite inside `exp_by_neg_x(...)` that materially improved the hard part.
- Go to `docs/plans/0002-cyclotomic-unitary-kernel-design.md` when evaluating whether to keep a short run of hard-part intermediates in torus/compressed form for repeated `cyclotomic * unitary_inverse(cyclotomic)` products.

## Build Instructions

Requirements:

- Rust stable toolchain
- `cargo`, `rustfmt`, and `clippy`

Commands:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo doc --no-deps
```

For the local `midnight-proofs` patch itself:

```bash
(cd patches/midnight-proofs && cargo clippy --all-targets --all-features -- -D warnings)
```

## CI Status

Basic GitHub Actions CI is defined in `.github/workflows/ci.yml`.

The workflow currently runs:

- `cargo check --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
- `cargo doc --no-deps`

Run the same commands locally before opening a PR.

## Benchmarking

Criterion benchmarks are scaffolded under `crates/wrapper-tests/benches/` and currently cover the implemented BN254 sanity-check circuits:

- `field/`
- `ecc/`

Run them with:

```bash
cargo bench
```

## Profiling Layout Cost

For optimization work on the current narrow Groth16 BN254 slice, prefer the
layout-profiling CLI over ad hoc timing:

```bash
cargo run -p wrapper-cli -- profile-layout > baseline.tsv
```

Useful family-specific runs are:

```bash
cargo run -p wrapper-cli -- profile-layout --family groth16
cargo run -p wrapper-cli -- profile-layout --family outer
cargo run -p wrapper-cli -- profile-layout --family pairing-terms
cargo run -p wrapper-cli -- profile-layout --family public-inputs
cargo run -p wrapper-cli -- profile-layout --family blocks
```

Notes:

- output is TSV and intended for before/after diffs
- `public-inputs` and `blocks` are comparatively lighter
- `groth16`, `pairing-terms`, and `all` can take noticeably longer because they
  model large pairing-backed circuits
- if you inspect the output file before the command exits, it may look empty or
  incomplete; wait for the command to finish before comparing baselines
- `blocks` now includes `final exponentiation easy part`, `final exponentiation hard part`, and total `final exponentiation`
- for final-exponentiation work specifically, start with `docs/profiling.md` and `docs/midnight-optimizations.md`
- for the current local Midnight-backed optimization picture, start with `docs/midnight-optimizations.md`
- the current Groth16 verifier route also precomputes Miller-step line
  coefficients off-circuit for constant verifier-key G2 terms (`beta_g2`,
  `gamma_g2`, `delta_g2`), trading a larger prepared VK representation for
  lower circuit cost

## Implemented Primitive Layer

What works now:

- BN254 foreign-field values wrapped as `AssignedFp` over Midnight `FieldChip`
- Circuit-backed `fp add` and `fp mul` in Halo2 via `midnight-circuits`
- BN254 Fp2 values wrapped as `AssignedFp2 = (c0, c1)` with `u^2 = -1`
- Circuit-backed `fp2 add`, `fp2 sub`, `fp2 neg`, `fp2 mul`, and `fp2 square`
- BN254 Fp6 values wrapped as `AssignedFp6 = (c0, c1, c2)` with `c0 + c1 * v + c2 * v^2`
- BN254 Fp6 tower matching arkworks exactly: `Fp6 = Fp2[v] / (v^3 - (9 + u))`
- Circuit-backed `fp6 add`, `fp6 sub`, `fp6 neg`, `fp6 mul`, and `fp6 square`
- BN254 Fp12 values wrapped as `AssignedFp12 = (c0, c1)` with `c0 + c1 * w`
- BN254 Fp12 tower matching arkworks exactly: `Fp12 = Fp6[w] / (w^2 - v)` with quadratic nonresidue `v = Fp6(0, 1, 0)`
- Circuit-backed `fp12 add`, `fp12 sub`, `fp12 neg`, `fp12 mul`, and `fp12 square`
- Minimal BN254 G1 support wrapped as `AssignedG1` over Midnight `ForeignEccChip`
- Circuit-backed G1 addition plus coordinate-to-point on-curve enforcement
- Minimal BN254 G2 affine support wrapped as `AssignedG2Affine = (x, y)` over `AssignedFp2`
- Circuit-backed G2 affine negation plus twist on-curve validation
- Narrow BN254 G2 Jacobian support wrapped as `AssignedG2Projective = (X, Y, Z)` with affine model `x = X / Z^2`, `y = Y / Z^3`
- Circuit-backed `from_affine`, `neg`, `double`, and incomplete Jacobian-Jacobian `add`
- Miller-path G2 state wrapped as `AssignedG2MillerPoint = (X, Y, Z)` in homogeneous projective coordinates with affine model `x = X / Z`, `y = Y / Z`
- Miller-ready BN254 line coefficients wrapped as `AssignedG2LineCoeffs = (ell_0, ell_w, ell_vw)`
- Miller-loop accumulator state wrapped as `AssignedMillerAccumulator`
- Line-coefficient layout chosen for BN254 D-twist sparse `Fp12` consumption:
  `ell_0 * y_P + ell_w * x_P * w + ell_vw * v * w`
- Circuit-backed `double_with_line` and `mixed_add_with_line` following the same homogeneous-projective prepared-G2 formulas used by arkworks / Midnight for BN prepared-G2 generation
- The public consumption boundary is now `AssignedG2LineCoeffs -> AssignedMillerAccumulator::mul_by_line(...)`
- `AssignedFp12` stays relatively clean; sparse line evaluation remains an internal detail of the accumulator for now
- Narrow BN254 Miller-loop accumulation over the real fixed optimal-ate prepared schedule
- Narrow BN254 final exponentiation over Miller-loop output
- Narrow verifier-shaped BN254 pairing-product check with one shared final exponentiation
- First narrow Groth16 BN254 verifier slice with real `snarkjs` parsing, verifier-only IC accumulation, and end-to-end valid/invalid fixture coverage
- Tuple-based host/reference arithmetic for the BN254 tower is centralized under `wrapper-circuits/src/bn254/host/`
- Deterministic randomized tests against arkworks reference behavior
- Real row/layout measurements via `midnight_proofs::dev::cost_model`
- Small Criterion benchmark hooks over the actual Week 1 sanity circuits
- A canonical primitive registry in `wrapper-circuits/src/planning.rs` now drives measured primitive metadata for `wrapper-cli doctor` and `wrapper-cli bench-info`
- CLI reporting that reflects measured primitive layout data
- A single authoritative BN254 implementation path in `wrapper-circuits/src/bn254/` without leftover host-side compatibility modules

What still does not exist:

- G2 subgroup checks or scalar multiplication
- broad public pairing or multi-pairing APIs beyond the narrow pairing-check slice
- broad public MSM support
- generalized Groth16 verification beyond the first narrow BN254 slice
- broad wrapper verifier circuit composition
- production-focused optimization beyond the narrow implemented sanity circuits
- broad backend ecosystems beyond the current narrow `snarkjs` BN254 parser path

## Running the CLI

The CLI is intentionally small and honest about the current phase. In particular, `doctor` reports measured primitive layout metrics and the still-missing verifier pieces.

```bash
cargo run -p wrapper-cli -- about
cargo run -p wrapper-cli -- doctor
cargo run -p wrapper-cli -- profile-layout
cargo run -p wrapper-cli -- print-layout
cargo run -p wrapper-cli -- validate-config --config crates/wrapper-tests/fixtures/example-config.toml
cargo run -p wrapper-cli -- bench-info
cargo run -p wrapper-cli -- inspect-groth16-bundle --proof ... --public ... --vk ...
cargo run -p wrapper-cli -- plan-wrapper-job --proof ... --public ... --vk ...
cargo run -p wrapper-cli -- export-wrapper-job --proof ... --public ... --vk ...
cargo run -p wrapper-cli -- export-wrapper-package --proof ... --public ... --vk ...
cargo run -p wrapper-cli -- execute-wrapper-stub --proof ... --public ... --vk ...
cargo run -p wrapper-cli -- execute-wrapper-direct --proof ... --public ... --vk ...
```

## Direct Execution Commands

The current direct BN254-hosted smoke-path commands are:

```bash
cargo run -q -p wrapper-cli -- execute-wrapper-direct-setup \
  --id circom-multiplier2 \
  --proof crates/wrapper-tests/fixtures/groth16/circom_multiplier2/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/circom_multiplier2/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/circom_multiplier2/verification_key.json \
  --backend midnight-bn254-host \
  --output /home/lorenzo/direct-setup-smoke.json

cargo run -q -p wrapper-cli -- execute-wrapper-direct-prove \
  --id circom-multiplier2 \
  --proof crates/wrapper-tests/fixtures/groth16/circom_multiplier2/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/circom_multiplier2/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/circom_multiplier2/verification_key.json \
  --backend midnight-bn254-host \
  --setup /home/lorenzo/direct-setup-smoke.json \
  --output /home/lorenzo/direct-prove-smoke.json

cargo run -q -p wrapper-cli -- execute-wrapper-direct-prove-trace \
  --id circom-multiplier2 \
  --proof crates/wrapper-tests/fixtures/groth16/circom_multiplier2/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/circom_multiplier2/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/circom_multiplier2/verification_key.json \
  --backend midnight-bn254-host \
  --setup /home/lorenzo/direct-setup-smoke.json \
  --output /home/lorenzo/direct-prove-trace-smoke.bin

cargo run -q -p wrapper-cli -- execute-wrapper-direct-prove-finalize \
  --id circom-multiplier2 \
  --proof crates/wrapper-tests/fixtures/groth16/circom_multiplier2/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/circom_multiplier2/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/circom_multiplier2/verification_key.json \
  --backend midnight-bn254-host \
  --setup /home/lorenzo/direct-setup-smoke.json \
  --trace /home/lorenzo/direct-prove-trace-smoke.bin \
  --output /home/lorenzo/direct-prove-finalized-smoke.json

cargo run -q -p wrapper-cli -- execute-wrapper-direct-verify \
  --id circom-multiplier2 \
  --proof crates/wrapper-tests/fixtures/groth16/circom_multiplier2/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/circom_multiplier2/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/circom_multiplier2/verification_key.json \
  --backend midnight-bn254-host \
  --bundle /home/lorenzo/direct-prove-finalized-smoke.json
```

Notes:

- artifact hygiene rule:
  - after any code or patch change that affects `execute-wrapper-direct-setup`,
    delete setup artifacts produced before that change before trusting a new
    prove/finalize measurement
  - after any code or patch change that affects
    `execute-wrapper-direct-prove-trace`, delete previously materialized trace
    artifacts and trace logs before rerunning
  - after any code or patch change that affects
    `execute-wrapper-direct-prove-finalize`, delete previously materialized
    finalized proof bundles and finalize logs before rerunning
- rerun `execute-wrapper-direct-prove-trace` after any patch change that modifies the persisted trace format
- when rerunning `execute-wrapper-direct-prove-trace` after one failed or obsolete attempt, remove the previous trace artifact and trace log first so the next run starts from a clean slate
- `execute-wrapper-direct-prove-finalize` is the current active memory-reduction work item
- `execute-wrapper-direct-prove-finalize` also exposes an optional `--h-poly-row-chunk-size` override, but it should usually be omitted unless you are actively tuning memory usage after an OOM
- that flag now accepts a base-2 exponent, not a raw row count:
  - `16` means `2^16 = 65536` rows
  - `15` means `2^15 = 32768` rows
- use it when `prove-finalize` still fails during the chunked permutation path inside `h_poly`
  and you want to trade more runtime for lower peak memory

## Development Workflow

1. Keep domain modeling in `wrapper-core` first.
2. Add Halo2-facing types in `wrapper-circuits` only when they truly require circuit integration, and prefer extending `src/bn254/` over reintroducing parallel primitive wrappers.
3. Put proof artifact loading and ecosystem adapters in `wrapper-backends`.
4. Expose orchestration and diagnostics through `wrapper-cli`.
5. Add regression coverage in `wrapper-tests` before growing implementation scope.

For the current narrow primitive-plus-first-verifier phase, prefer correctness
and measured layout visibility over optimization.

## Roadmap / Phases

- Initialization: workspace scaffold, docs, CLI, placeholders, tests
- Stage 1 / Week 1: Midnight-backed BN254 `fp add` / `fp mul`, minimal G1 addition, arkworks sanity checks, layout visibility
- Stage 1 / Week 2 slice 1: BN254 `fp2` arithmetic over the existing Midnight-backed `AssignedFp` layer, with measured add/mul/square costs
- Stage 1 / Week 2 slice 2: minimal BN254 G2 affine assignment, equality, negation, and twist on-curve validation
- Stage 1 / Week 2 slice 3: narrow BN254 G2 Jacobian projective embedding, negation, doubling, incomplete addition, and cost visibility
- Stage 1 / early Week 3 slice: BN254 `fp6` arithmetic over the existing `AssignedFp2` layer, with measured add/mul/square costs
- Stage 1 / Week 3 slice: BN254 `fp12` arithmetic over the existing `AssignedFp6` layer, with measured add/mul/square costs
- Stage 1 / Week 3 slice: BN254 G2 `double_with_line` / `mixed_add_with_line` extraction with Miller-ready sparse coefficient layout
- Stage 1 / Week 4 slice: narrow pairing core with real Miller loop, final exponentiation, and verifier-shaped pairing check
- Stage 1 / Week 5 slice: first narrow Groth16 BN254 verifier path with real fixtures and one end-to-end verifier equation reduction
- Later pairing work: foreign field and pairing-related gadget research
- Later wrapper verifier work: broader Groth16 verifier logic inside the outer proof system
- Possible Cardano integration: ecosystem-specific packaging, artifacts, and engineering constraints
- Possible Semaphore migration case study: testing a migrated application-shaped circuit use case

See [docs/roadmap.md](docs/roadmap.md) for more detail.

## Design Principles

- Keep core architecture explicit and boring.
- Avoid fake implementations and cryptographic theater.
- Separate domain concerns from Halo2 concerns.
- Isolate backend adapters so parser churn does not infect circuit code.
- Document intent before implementation complexity grows.
- Add dependencies conservatively.

## Testing Strategy

Current strategy:

- Compile all crates
- Validate CLI behavior
- Test configuration parsing and current metadata/status behavior
- Validate BN254 field and G1 behavior against arkworks
- Keep small Midnight-backed sanity benchmarks runnable so future performance work has a consistent home

Future strategy:

- Fixture-driven integration tests in `wrapper-tests`
- Golden-file checks for artifact parsing
- Cross-crate contract tests between domain, backend, and circuit layers
- Eventually, proof-generation and verification test matrices once cryptographic code exists

## Non-Goals for This Phase

- Shipping a usable wrapper proof system
- Implementing full G2 arithmetic, broader pairings, or generalized Groth16 verification
- Selecting a final proving backend
- Claiming compatibility with production proof artifacts

## Disclaimer

This repository now contains a circuit-backed BN254 primitive layer using `midnight-circuits` and `midnight-proofs`, organized under `wrapper-circuits/src/bn254/`, together with a first narrow Groth16 BN254 verifier slice in `wrapper-circuits/src/groth16.rs` and `wrapper-backends/src/snarkjs.rs`. The Fp6 and Fp12 layers support `add`, `sub`, `neg`, `mul`, and `square` over the arkworks-compatible BN254 tower; the Jacobian G2 layer supports non-identity `from_affine`, `neg`, `double`, and incomplete `add`; the Miller-path layer supports non-identity `double_with_line` and `mixed_add_with_line` with sparse `Fp12`-facing coefficients; and the verifier slice now supports real snarkjs proof/VK parsing, verifier-only IC accumulation, and one end-to-end pairing-product-check verification path. The repository still does not include subgroup checks, broad scalar multiplication, generalized verifier frameworks, proof generation, or a production wrapper verifier circuit. Current Criterion benchmarks are sanity-check hooks over small implemented circuits and should not be read as production cryptographic performance claims.
