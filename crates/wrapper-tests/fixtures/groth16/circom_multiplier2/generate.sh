#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

cp "$SCRIPT_DIR/multiplier2.circom" "$WORK_DIR/"
cp "$SCRIPT_DIR/input.json" "$WORK_DIR/"

circom "$WORK_DIR/multiplier2.circom" --r1cs --wasm --sym -o "$WORK_DIR"

pushd "$WORK_DIR" >/dev/null
npx -y snarkjs powersoftau new bn128 12 pot12_0000.ptau
npx -y snarkjs powersoftau prepare phase2 pot12_0000.ptau pot12_final.ptau
npx -y snarkjs groth16 setup multiplier2.r1cs pot12_final.ptau multiplier2_0000.zkey
npx -y snarkjs zkey export verificationkey multiplier2_0000.zkey verification_key.json
npx -y snarkjs groth16 fullprove input.json multiplier2_js/multiplier2.wasm multiplier2_0000.zkey proof.json public.json
popd >/dev/null

cp "$WORK_DIR/verification_key.json" "$SCRIPT_DIR/"
cp "$WORK_DIR/proof.json" "$SCRIPT_DIR/"
cp "$WORK_DIR/public.json" "$SCRIPT_DIR/"
