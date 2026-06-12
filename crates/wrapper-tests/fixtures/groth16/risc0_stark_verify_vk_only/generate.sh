#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../../.." && pwd)"
RISC0_REPO="${RISC0_REPO:-$REPO_ROOT/../risc0}"
WORK_DIR="$(mktemp -d)"
RESTORE_FILES=()
cleanup() {
  for file in "${RESTORE_FILES[@]}"; do
    local backup_name
    backup_name="$(echo "$file" | sed 's#[/:]#_#g').bak"
    if [ -f "$WORK_DIR/$backup_name" ]; then
      cp "$WORK_DIR/$backup_name" "$file"
    fi
  done
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

if [ ! -d "$RISC0_REPO" ]; then
  echo "missing sibling risc0 repo: $RISC0_REPO" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required for the RISC Zero shrink-wrap path" >&2
  exit 1
fi

if ! command -v circom >/dev/null 2>&1; then
  echo "circom is required" >&2
  exit 1
fi

ensure_real_lfs_blob() {
  local target_path="$1"
  local media_url="$2"

  if xz -t "$target_path" >/dev/null 2>&1; then
    return
  fi

  local backup_name
  backup_name="$(echo "$target_path" | sed 's#[/:]#_#g').bak"
  cp "$target_path" "$WORK_DIR/$backup_name"
  RESTORE_FILES+=("$target_path")
  curl -L --fail --silent --show-error -o "$target_path" "$media_url"
  xz -t "$target_path" >/dev/null
}

SNARKJS="${SNARKJS:-npx --yes --package snarkjs@0.7.6 snarkjs}"
HARNESS_DIR="$WORK_DIR/harness"
OUT_DIR="$WORK_DIR/out"
TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/target/risc0-stark-verify-poc}"

mkdir -p "$HARNESS_DIR/src" "$OUT_DIR"

ensure_real_lfs_blob \
  "$RISC0_REPO/risc0/circuit/recursion-zkrs/src/zkrs.tar.xz" \
  "https://media.githubusercontent.com/media/risc0/risc0/main/risc0/circuit/recursion-zkrs/src/zkrs.tar.xz"
ensure_real_lfs_blob \
  "$RISC0_REPO/risc0/circuit/recursion-povw-zkrs/src/zkrs.tar.xz" \
  "https://media.githubusercontent.com/media/risc0/risc0/main/risc0/circuit/recursion-povw-zkrs/src/zkrs.tar.xz"
ensure_real_lfs_blob \
  "$RISC0_REPO/risc0/circuit/keccak/src/prove/keccak_lift_14.zkr.xz" \
  "https://media.githubusercontent.com/media/risc0/risc0/main/risc0/circuit/keccak/src/prove/keccak_lift_14.zkr.xz"
ensure_real_lfs_blob \
  "$RISC0_REPO/risc0/circuit/keccak/src/prove/keccak_lift_15.zkr.xz" \
  "https://media.githubusercontent.com/media/risc0/risc0/main/risc0/circuit/keccak/src/prove/keccak_lift_15.zkr.xz"
ensure_real_lfs_blob \
  "$RISC0_REPO/risc0/circuit/keccak/src/prove/keccak_lift_16.zkr.xz" \
  "https://media.githubusercontent.com/media/risc0/risc0/main/risc0/circuit/keccak/src/prove/keccak_lift_16.zkr.xz"
ensure_real_lfs_blob \
  "$RISC0_REPO/risc0/circuit/keccak/src/prove/keccak_lift_17.zkr.xz" \
  "https://media.githubusercontent.com/media/risc0/risc0/main/risc0/circuit/keccak/src/prove/keccak_lift_17.zkr.xz"
ensure_real_lfs_blob \
  "$RISC0_REPO/risc0/circuit/keccak/src/prove/keccak_lift_18.zkr.xz" \
  "https://media.githubusercontent.com/media/risc0/risc0/main/risc0/circuit/keccak/src/prove/keccak_lift_18.zkr.xz"

cat >"$HARNESS_DIR/Cargo.toml" <<EOF
[package]
name = "risc0-stark-verify-poc"
version = "0.1.0"
edition = "2024"

[dependencies]
anyhow = "1.0"
hello-world-methods = { path = "$RISC0_REPO/examples/hello-world/methods" }
num-bigint = "0.4"
risc0-groth16 = { path = "$RISC0_REPO/risc0/groth16", features = ["std"] }
risc0-zkvm = { path = "$RISC0_REPO/risc0/zkvm", features = ["client", "std"] }
serde_json = "1.0"
EOF

cat >"$HARNESS_DIR/src/main.rs" <<'EOF'
use anyhow::{Context, Result};
use hello_world_methods::MULTIPLY_ELF;
use num_bigint::BigUint;
use risc0_groth16::Seal;
use risc0_zkvm::{
    ExecutorEnv, Groth16ReceiptVerifierParameters, Prover, ProverOpts, default_prover,
    sha::Digestible,
};
use serde_json::to_string_pretty;
use std::{env, fs, path::PathBuf};

fn digest_halves_decimal(digest: risc0_zkvm::sha::Digest) -> [String; 2] {
    let big_endian: Vec<u8> = digest.as_bytes().iter().rev().copied().collect();
    let middle = big_endian.len() / 2;
    let (low_half, high_half) = big_endian.split_at(middle);
    [
        BigUint::from_bytes_be(high_half).to_str_radix(10),
        BigUint::from_bytes_be(low_half).to_str_radix(10),
    ]
}

fn digest_decimal(digest: risc0_zkvm::sha::Digest) -> String {
    let big_endian: Vec<u8> = digest.as_bytes().iter().rev().copied().collect();
    BigUint::from_bytes_be(&big_endian).to_str_radix(10)
}

fn bytes_be_to_decimal(bytes: &[u8]) -> String {
    BigUint::from_bytes_be(bytes).to_str_radix(10)
}

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::args().nth(1).context("missing output directory")?);
    fs::create_dir_all(&out_dir)?;

    unsafe { env::set_var("RISC0_PROVER", "ipc") };

    let env = ExecutorEnv::builder()
        .write(&17_u64)?
        .write(&23_u64)?
        .build()?;

    let info = default_prover().prove_with_opts(env, MULTIPLY_ELF, &ProverOpts::groth16())?;
    let groth16 = info
        .receipt
        .inner
        .groth16()
        .context("expected a groth16 receipt")?
        .clone();
    let seal = Seal::decode(&groth16.seal)?;

    let proof = serde_json::json!({
        "pi_a": [
            bytes_be_to_decimal(&seal.a[0]),
            bytes_be_to_decimal(&seal.a[1]),
            "1"
        ],
        "pi_b": [
            [
                bytes_be_to_decimal(&seal.b[0][1]),
                bytes_be_to_decimal(&seal.b[0][0])
            ],
            [
                bytes_be_to_decimal(&seal.b[1][1]),
                bytes_be_to_decimal(&seal.b[1][0])
            ],
            ["1", "0"]
        ],
        "pi_c": [
            bytes_be_to_decimal(&seal.c[0]),
            bytes_be_to_decimal(&seal.c[1]),
            "1"
        ],
        "protocol": "groth16",
        "curve": "bn128"
    });
    fs::write(
        out_dir.join("proof.json"),
        format!("{}\n", to_string_pretty(&proof)?),
    )?;

    let params = Groth16ReceiptVerifierParameters::default();
    let control_root = digest_halves_decimal(params.control_root);
    let claim_digest = digest_halves_decimal(groth16.claim.digest());
    let bn254_control_id = digest_decimal(params.bn254_control_id);

    let public_inputs = vec![
        control_root[0].clone(),
        control_root[1].clone(),
        claim_digest[0].clone(),
        claim_digest[1].clone(),
        bn254_control_id,
    ];
    fs::write(
        out_dir.join("public.json"),
        format!("{}\n", to_string_pretty(&public_inputs)?),
    )?;

    Ok(())
}
EOF

pushd "$HARNESS_DIR" >/dev/null
RISC0_PROVER=ipc CARGO_TARGET_DIR="$TARGET_DIR" cargo run --release -- "$OUT_DIR"
popd >/dev/null

$SNARKJS groth16 verify \
  "$SCRIPT_DIR/verification_key.json" \
  "$OUT_DIR/public.json" \
  "$OUT_DIR/proof.json"

cp "$OUT_DIR/proof.json" "$SCRIPT_DIR/"
cp "$OUT_DIR/public.json" "$SCRIPT_DIR/"
