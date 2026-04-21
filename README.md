# Halo2 Wrapper Workspace

This repository is a Rust workspace for a staged research and engineering effort around a Halo2-based outer proof system that may eventually verify Groth16 BN254 proofs inside a Halo2 wrapper.

The current phase is still intentionally narrow, but it is no longer just repository bootstrap: the project now includes a circuit-backed BN254 primitive layer built on `midnight-circuits` and `midnight-proofs`, together with CI, benchmarks, CLI diagnostics, and contributor documentation. Week 2 has started with a first Fp2 slice, while G2 and pairings remain out of scope.

## Current Status

What the repository currently contains:

- A multi-crate Rust workspace with explicit boundaries between domain logic, circuit-facing code, backend adapters, CLI tooling, and test harness code.
- Week 1 BN254 foundations in `wrapper-circuits`, organized under `src/bn254/`: Midnight-backed foreign-field and G1 circuits with real layout measurements.
- A first Week 2 BN254 Fp2 layer in `wrapper-circuits`, also organized under `src/bn254/`, built from two circuit-backed `AssignedFp` coordinates.
- Placeholder outer-wrapper planning and backend integration boundaries that are honest about what is still missing.
- Contributor-oriented documentation covering architecture, roadmap, and initial design decisions.
- A `wrapper-cli` binary with honest developer commands for environment inspection and configuration validation.

What is explicitly not implemented yet:

- Pairing gadgets or pairing arithmetic
- Groth16 verifier logic
- G2, Fp6, or Fp12 support
- Real backend adapters to arkworks, Midnight, `blst`, or `snarkjs`
- Cryptographic soundness claims of any kind

This repository now includes Week 1 arithmetic foundations, but it is still far from a Groth16 wrapper verifier.

## Planned Architecture

The intended shape of the project is:

- `wrapper-core`: domain-oriented types, traits, config, errors, metadata, and public architectural contracts
- `wrapper-circuits`: Halo2-facing circuits, current Midnight-backed BN254 primitive layer, layout descriptions, and future gadget integration points
- `wrapper-backends`: artifact loading, parser adapters, proof/VK material ingestion, and future external backend bridges
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
│   ├── roadmap.md
│   └── decisions/0001-initial-workspace-structure.md
└── crates/
    ├── wrapper-core/
    ├── wrapper-circuits/
    ├── wrapper-backends/
    ├── wrapper-cli/
    └── wrapper-tests/
```

## Build Instructions

Requirements:

- Rust stable toolchain
- `cargo`, `rustfmt`, and `clippy`

Commands:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --no-deps
```

## CI Status

Basic GitHub Actions CI is defined in `.github/workflows/ci.yml`.

The workflow currently runs:

- `cargo check --workspace`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
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

## Implemented Primitive Layer

What works now:

- BN254 foreign-field values wrapped as `AssignedFp` over Midnight `FieldChip`
- Circuit-backed `fp add` and `fp mul` in Halo2 via `midnight-circuits`
- BN254 Fp2 values wrapped as `AssignedFp2 = (c0, c1)` with `u^2 = -1`
- Circuit-backed `fp2 add`, `fp2 sub`, `fp2 neg`, `fp2 mul`, and `fp2 square`
- Minimal BN254 G1 support wrapped as `AssignedG1` over Midnight `ForeignEccChip`
- Circuit-backed G1 addition plus coordinate-to-point on-curve enforcement
- Deterministic randomized tests against arkworks reference behavior
- Real row/layout measurements via `midnight_proofs::dev::cost_model`
- Small Criterion benchmark hooks over the actual Week 1 sanity circuits
- CLI reporting that reflects measured primitive layout data
- A single authoritative BN254 implementation path in `wrapper-circuits/src/bn254/` without leftover host-side compatibility modules

What still does not exist:

- pairings
- G2
- Fp6/Fp12
- MSM
- Groth16 verification
- wrapper verifier logic
- production-focused optimization or proof-system integration work
- real proof or verification-key backend adapters

## Running the CLI

The CLI is intentionally small and honest about the current phase. In particular, `doctor` reports measured primitive layout metrics and the still-missing verifier pieces.

```bash
cargo run -p wrapper-cli -- about
cargo run -p wrapper-cli -- doctor
cargo run -p wrapper-cli -- print-layout
cargo run -p wrapper-cli -- validate-config --config crates/wrapper-tests/fixtures/example-config.toml
cargo run -p wrapper-cli -- bench-info
```

## Development Workflow

1. Keep domain modeling in `wrapper-core` first.
2. Add Halo2-facing types in `wrapper-circuits` only when they truly require circuit integration, and prefer extending `src/bn254/` over reintroducing parallel primitive wrappers.
3. Put proof artifact loading and ecosystem adapters in `wrapper-backends`.
4. Expose orchestration and diagnostics through `wrapper-cli`.
5. Add regression coverage in `wrapper-tests` before growing implementation scope.

For the current primitive-foundation phase, prefer correctness and measured layout visibility over optimization.

## Roadmap / Phases

- Initialization: workspace scaffold, docs, CLI, placeholders, tests
- Stage 1 / Week 1: current phase; Midnight-backed BN254 `fp add` / `fp mul`, minimal G1 addition, arkworks sanity checks, layout visibility
- Stage 1 / Week 2 slice 1: BN254 `fp2` arithmetic over the existing Midnight-backed `AssignedFp` layer, with measured add/mul/square costs
- Later pairing work: foreign field and pairing-related gadget research
- Later wrapper verifier work: Groth16 verifier logic inside the outer proof system
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
- Implementing pairings, G2, Fp6/Fp12, or Groth16 verification
- Selecting a final proving backend
- Claiming compatibility with production proof artifacts

## Disclaimer

This repository now contains a circuit-backed BN254 primitive layer using `midnight-circuits` and `midnight-proofs`, organized under `wrapper-circuits/src/bn254/`, including a first Week 2 Fp2 slice. It still does not implement G2, pairings, Groth16 verification, or a wrapper verifier circuit. Current Criterion benchmarks are sanity-check hooks over small implemented circuits and should not be read as production cryptographic performance claims.
