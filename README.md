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
