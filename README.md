# Halo2 Wrapper Workspace

This repository is a Rust workspace for a Halo2/Midnight outer proof system that wraps Groth16 BN254 proofs. The current implementation is intentionally narrow and focused on the existing BN254 primitive layer, the Groth16 verifier slice, layout profiling, and the direct `setup -> prove -> verify` execution flow for the committed `circom_multiplier2` and `semaphore` fixtures.

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
3. generate a new outer Halo2 proof, optionally hosted on BLS12-381

The current parser expects the standard `snarkjs` artifact triple:

- `proof.json`
- `public.json`
- `verification_key.json`

Current assumptions:

- the inner proof system is Groth16 over the curve family emitted by `snarkjs`
  as `bn128`, i.e. the BN254 family used by this repo
- artifacts must already be normalized into the standard `snarkjs` JSON shape
- the current outer public statement is the ordered inner public-input vector,
  optionally with semantic names supplied on the CLI

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
artifact set, using the BLS12-381 outer host lane:

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

## `circom_multiplier2` Execution

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
