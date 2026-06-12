# Halo2 Wrapper Workspace

This repository is a Rust workspace for a Halo2/Midnight outer proof system
that wraps Groth16 BN254 proofs. The current implementation is intentionally
narrow and focused on the existing BN254 primitive layer, the Groth16 verifier
slice, layout profiling, and the direct `setup -> prove -> verify` execution
flow for the committed `circom_multiplier2` and `semaphore` fixtures.

Current outer-lane policy:

- the official outer lane is `BLS12-381`
- the BN254-hosted outer lane remains available as a compatibility/testing lane
- the canonical public statement and VK commitment semantics are shared across
  both lanes, but operator-facing workflows should default mentally to
  `BLS12-381`

## Build Instructions

Build the full workspace with:

```bash
cargo build --workspace
```

Useful verification commands during development:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Profiling

Run the layout profiler with:

```bash
cargo run -p wrapper-cli -- profile-layout
```

Useful profiling variants:

```bash
cargo run -p wrapper-cli -- profile-layout --family groth16
cargo run -p wrapper-cli -- profile-layout --family outer
cargo run -p wrapper-cli -- profile-layout --family pairing-terms
cargo run -p wrapper-cli -- profile-layout --family public-inputs
cargo run -p wrapper-cli -- profile-layout --family blocks
```

## Using It From Circom and snarkJS

This repository does not convert a Groth16 BN254 proof into another Groth16
proof on BLS12-381.

What it does is:

1. parse a Groth16 BN254 proof generated from `circom` and `snarkjs`
2. build a Halo2 outer circuit that verifies that inner BN254 proof
3. generate a new outer Halo2 proof, with `BLS12-381` as the official hosted
   lane

The current parser expects the standard `snarkjs` artifact triple:

- `proof.json`
- `public.json`
- `verification_key.json`

Current assumptions:

- the inner proof system is Groth16 over the curve family emitted by `snarkjs`
  as `bn128`, i.e. the BN254 family used by this repo
- artifacts must already be normalized into the standard `snarkjs` JSON shape
- the current outer public statement is the ordered inner public-input vector,
  plus a public VK commitment, optionally with semantic names supplied on the
  CLI

In other words, the operational flow is:

`circom` circuit -> `snarkjs` Groth16 BN254 artifacts -> wrapper parser ->
canonical outer circuit -> outer Halo2 proof

### Minimal Circom / snarkJS Workflow

Starting from a Circom circuit, the shortest path is:

```bash
circom my_circuit.circom --r1cs --wasm --sym -o build
snarkjs groth16 setup build/my_circuit.r1cs <ptau> build/my_circuit_0000.zkey
snarkjs zkey export verificationkey build/my_circuit_0000.zkey verification_key.json
snarkjs groth16 fullprove input.json build/my_circuit_js/my_circuit.wasm build/my_circuit_0000.zkey proof.json public.json
snarkjs groth16 verify verification_key.json public.json proof.json
```

Once those three JSON artifacts exist, this repository can wrap them.

### Generic Wrapper Flow on BLS12-381

The example below shows the generic direct lane for a real Circom/snarkJS
artifact set, using the official BLS12-381 outer host lane:

```bash
mkdir -p artifacts/my-circuit

cargo run -p wrapper-cli -- execute-wrapper-direct-setup \
  --id my-circuit \
  --proof /absolute/path/to/proof.json \
  --public /absolute/path/to/public.json \
  --vk /absolute/path/to/verification_key.json \
  --backend midnight-bls12381-host \
  --output artifacts/my-circuit/setup.json

cargo run -p wrapper-cli -- execute-wrapper-direct-prove-trace \
  --id my-circuit \
  --proof /absolute/path/to/proof.json \
  --public /absolute/path/to/public.json \
  --vk /absolute/path/to/verification_key.json \
  --backend midnight-bls12381-host \
  --setup artifacts/my-circuit/setup.json \
  --output artifacts/my-circuit/trace.bin

cargo run -p wrapper-cli -- execute-wrapper-direct-prove-finalize \
  --id my-circuit \
  --proof /absolute/path/to/proof.json \
  --public /absolute/path/to/public.json \
  --vk /absolute/path/to/verification_key.json \
  --backend midnight-bls12381-host \
  --setup artifacts/my-circuit/setup.json \
  --trace artifacts/my-circuit/trace.bin \
  --output artifacts/my-circuit/proof-bundle.json

cargo run -p wrapper-cli -- execute-wrapper-direct-verify \
  --id my-circuit \
  --proof /absolute/path/to/proof.json \
  --public /absolute/path/to/public.json \
  --vk /absolute/path/to/verification_key.json \
  --backend midnight-bls12381-host \
  --bundle artifacts/my-circuit/proof-bundle.json
```

If the inner public inputs have semantic names, add repeated
`--public-input-name <name>` flags in the same order as `public.json`.

### Compatibility Note About BN254 Outer

The repository still contains a BN254-hosted outer lane for compatibility,
profiling comparisons, and regression work.

That lane is not the official operator target.

If the task is about the production-facing or Cardano-facing outer flow,
prefer:

- `--backend midnight-bls12381-host`

## `circom_multiplier2` Execution

The commands below use the retained BN254-hosted compatibility lane because
they are historical performance and debugging references for the smaller
fixture.

For production-facing or Cardano-facing workflows, prefer the BLS12-381
pattern shown above unless you are intentionally exercising the compatibility
lane.

```bash
mkdir -p artifacts/direct-profile-circom-multiplier2

cargo run -p wrapper-cli -- execute-wrapper-direct-setup \
  --id circom-multiplier2 \
  --proof crates/wrapper-tests/fixtures/groth16/circom_multiplier2/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/circom_multiplier2/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/circom_multiplier2/verification_key.json \
  --backend midnight-bn254-host \
  --output artifacts/direct-profile-circom-multiplier2/setup.json

cargo run -p wrapper-cli -- execute-wrapper-direct-prove-trace \
  --id circom-multiplier2 \
  --proof crates/wrapper-tests/fixtures/groth16/circom_multiplier2/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/circom_multiplier2/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/circom_multiplier2/verification_key.json \
  --backend midnight-bn254-host \
  --setup artifacts/direct-profile-circom-multiplier2/setup.json \
  --output artifacts/direct-profile-circom-multiplier2/trace.bin

cargo run -p wrapper-cli -- execute-wrapper-direct-prove-finalize \
  --id circom-multiplier2 \
  --proof crates/wrapper-tests/fixtures/groth16/circom_multiplier2/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/circom_multiplier2/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/circom_multiplier2/verification_key.json \
  --backend midnight-bn254-host \
  --setup artifacts/direct-profile-circom-multiplier2/setup.json \
  --trace artifacts/direct-profile-circom-multiplier2/trace.bin \
  --output artifacts/direct-profile-circom-multiplier2/proof-bundle.json

cargo run -p wrapper-cli -- execute-wrapper-direct-verify \
  --id circom-multiplier2 \
  --proof crates/wrapper-tests/fixtures/groth16/circom_multiplier2/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/circom_multiplier2/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/circom_multiplier2/verification_key.json \
  --backend midnight-bn254-host \
  --bundle artifacts/direct-profile-circom-multiplier2/proof-bundle.json
```

## `semaphore` Execution

```bash
mkdir -p artifacts/direct-profile-semaphore

cargo run -p wrapper-cli -- execute-wrapper-direct-setup \
  --id semaphore-depth-10 \
  --proof crates/wrapper-tests/fixtures/groth16/semaphore/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/semaphore/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/semaphore/verification_key.json \
  --public-input-name merkle_root \
  --public-input-name nullifier \
  --public-input-name message_hash \
  --public-input-name scope_hash \
  --backend midnight-bls12381-host \
  --output artifacts/direct-profile-semaphore/semaphore-setup.json

cargo run -p wrapper-cli -- execute-wrapper-direct-prove-trace \
  --id semaphore-depth-10 \
  --proof crates/wrapper-tests/fixtures/groth16/semaphore/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/semaphore/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/semaphore/verification_key.json \
  --public-input-name merkle_root \
  --public-input-name nullifier \
  --public-input-name message_hash \
  --public-input-name scope_hash \
  --backend midnight-bls12381-host \
  --log-mode efficient \
  --setup artifacts/direct-profile-semaphore/semaphore-setup.json \
  --output artifacts/direct-profile-semaphore/semaphore-trace.bin

cargo run -p wrapper-cli -- execute-wrapper-direct-prove-finalize \
  --id semaphore-depth-10 \
  --proof crates/wrapper-tests/fixtures/groth16/semaphore/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/semaphore/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/semaphore/verification_key.json \
  --public-input-name merkle_root \
  --public-input-name nullifier \
  --public-input-name message_hash \
  --public-input-name scope_hash \
  --backend midnight-bls12381-host \
  --log-mode efficient \
  --setup artifacts/direct-profile-semaphore/semaphore-setup.json \
  --trace artifacts/direct-profile-semaphore/semaphore-trace.bin \
  --h-poly-row-chunk-size 13 \
  --output artifacts/direct-profile-semaphore/semaphore-proof-bundle.json

cargo run -p wrapper-cli -- execute-wrapper-direct-verify \
  --id semaphore-depth-10 \
  --proof crates/wrapper-tests/fixtures/groth16/semaphore/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/semaphore/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/semaphore/verification_key.json \
  --public-input-name merkle_root \
  --public-input-name nullifier \
  --public-input-name message_hash \
  --public-input-name scope_hash \
  --backend midnight-bls12381-host \
  --bundle artifacts/direct-profile-semaphore/semaphore-proof-bundle.json
```

## `risc0_stark_verify` Execution

The RISC Zero fixture depends on a sibling clone of `risc0` at `../risc0` by
default and uses the upstream shrink-wrap path to materialize the Groth16
artifacts locally.

Prerequisites for the RISC Zero fixture flow:

- a sibling clone of `risc0` at `../risc0`, or `RISC0_REPO=/absolute/path/to/risc0`
- `cargo`
- `curl`
- `docker`
- `circom`
- network access, because the helper script repairs a small set of required
  upstream Git LFS blobs on demand and may need to fetch RISC Zero toolchain
  artifacts
- the RISC Zero guest Rust toolchain installed through `rzup`
- a compatible `r0vm` available either on `PATH` or through
  `RISC0_SERVER_PATH`

Recommended one-time setup from the sibling `risc0` checkout:

```bash
cargo run --manifest-path ../risc0/rzup/Cargo.toml -- install rust
```

Important compatibility note:

- the `r0vm` binary used by the fixture should come from the same
  `risc0` code line as the sibling checkout
- if `r0vm` is not on `PATH`, set `RISC0_SERVER_PATH=/absolute/path/to/r0vm`
  before running `generate.sh`

First generate the fixture artifacts:

```bash
cd crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only
./generate.sh
cd /absolute/path/to/halo2-groth16-bn254-to-bls12-381
```

Then run the wrapper flow:

```bash
mkdir -p artifacts/direct-profile-risc0-stark-verify

cargo run -p wrapper-cli -- execute-wrapper-direct-setup \
  --id risc0-stark-verify \
  --proof crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/verification_key.json \
  --public-input-name control_root_0 \
  --public-input-name control_root_1 \
  --public-input-name claim_digest_0 \
  --public-input-name claim_digest_1 \
  --public-input-name bn254_control_id \
  --backend midnight-bls12381-host \
  --output artifacts/direct-profile-risc0-stark-verify/risc0-stark-verify-setup.json

cargo run -p wrapper-cli -- execute-wrapper-direct-prove-trace \
  --id risc0-stark-verify \
  --proof crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/verification_key.json \
  --public-input-name control_root_0 \
  --public-input-name control_root_1 \
  --public-input-name claim_digest_0 \
  --public-input-name claim_digest_1 \
  --public-input-name bn254_control_id \
  --backend midnight-bls12381-host \
  --log-mode efficient \
  --setup artifacts/direct-profile-risc0-stark-verify/risc0-stark-verify-setup.json \
  --output artifacts/direct-profile-risc0-stark-verify/risc0-stark-verify-trace.bin

cargo run -p wrapper-cli -- execute-wrapper-direct-prove-finalize \
  --id risc0-stark-verify \
  --proof crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/verification_key.json \
  --public-input-name control_root_0 \
  --public-input-name control_root_1 \
  --public-input-name claim_digest_0 \
  --public-input-name claim_digest_1 \
  --public-input-name bn254_control_id \
  --backend midnight-bls12381-host \
  --log-mode efficient \
  --setup artifacts/direct-profile-risc0-stark-verify/risc0-stark-verify-setup.json \
  --trace artifacts/direct-profile-risc0-stark-verify/risc0-stark-verify-trace.bin \
  --output artifacts/direct-profile-risc0-stark-verify/risc0-stark-verify-proof-bundle.json

cargo run -p wrapper-cli -- execute-wrapper-direct-verify \
  --id risc0-stark-verify \
  --proof crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/proof.json \
  --public crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/public.json \
  --vk crates/wrapper-tests/fixtures/groth16/risc0_stark_verify_vk_only/verification_key.json \
  --public-input-name control_root_0 \
  --public-input-name control_root_1 \
  --public-input-name claim_digest_0 \
  --public-input-name claim_digest_1 \
  --public-input-name bn254_control_id \
  --backend midnight-bls12381-host \
  --bundle artifacts/direct-profile-risc0-stark-verify/risc0-stark-verify-proof-bundle.json
```
