#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

PTAU_URL="https://storage.googleapis.com/zkevm/ptau/powersOfTau28_hez_final_21.ptau"
PTAU_CACHE_DIR="${HOME}/.cache/snarkjs"
PTAU_CACHE_PATH="${PTAU_CACHE_DIR}/powersOfTau28_hez_final_21.ptau"
SNARKJS="npx --yes --package snarkjs@0.7.6 snarkjs"

cp "$SCRIPT_DIR/source_email.eml" "$WORK_DIR/source_email.eml"
cp "$SCRIPT_DIR/email_verifier_header_only.circom" "$WORK_DIR/email_verifier_header_only.circom"
cp "$SCRIPT_DIR/generate_inputs.ts" "$WORK_DIR/generate_inputs.ts"

pushd "$WORK_DIR" >/dev/null
npm init -y >/dev/null
npm install \
  @zk-email/helpers@6.4.2 \
  @zk-email/circuits@6.3.4 \
  tsx >/dev/null

npx -y tsx generate_inputs.ts source_email.eml input.json

mkdir -p "$WORK_DIR/build"
mkdir -p "$PTAU_CACHE_DIR"

circom email_verifier_header_only.circom \
  -l "$WORK_DIR/node_modules" \
  --r1cs --wasm --sym --O0 \
  -o "$WORK_DIR/build"

$SNARKJS r1cs info "$WORK_DIR/build/email_verifier_header_only.r1cs"

if [ ! -f "$PTAU_CACHE_PATH" ]; then
  curl -L --fail --silent --show-error -o "$PTAU_CACHE_PATH" "$PTAU_URL"
fi

$SNARKJS file info "$PTAU_CACHE_PATH" >/dev/null

$SNARKJS groth16 setup \
  "$WORK_DIR/build/email_verifier_header_only.r1cs" \
  "$PTAU_CACHE_PATH" \
  "$WORK_DIR/email_verifier_header_only_0000.zkey"

$SNARKJS zkey export verificationkey \
  "$WORK_DIR/email_verifier_header_only_0000.zkey" \
  "$WORK_DIR/verification_key.json"

node "$WORK_DIR/build/email_verifier_header_only_js/generate_witness.js" \
  "$WORK_DIR/build/email_verifier_header_only_js/email_verifier_header_only.wasm" \
  "$WORK_DIR/input.json" \
  "$WORK_DIR/witness.wtns"

$SNARKJS groth16 prove \
  "$WORK_DIR/email_verifier_header_only_0000.zkey" \
  "$WORK_DIR/witness.wtns" \
  "$WORK_DIR/proof.json" \
  "$WORK_DIR/public.json"

$SNARKJS groth16 verify \
  "$WORK_DIR/verification_key.json" \
  "$WORK_DIR/public.json" \
  "$WORK_DIR/proof.json"
popd >/dev/null

cp "$WORK_DIR/input.json" "$SCRIPT_DIR/"
cp "$WORK_DIR/verification_key.json" "$SCRIPT_DIR/"
cp "$WORK_DIR/proof.json" "$SCRIPT_DIR/"
cp "$WORK_DIR/public.json" "$SCRIPT_DIR/"

node - "$SCRIPT_DIR/metadata.json" "$SCRIPT_DIR/public.json" <<'EOF'
const fs = require("node:fs");

const metadataPath = process.argv[2];
const publicPath = process.argv[3];

const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf8"));
const publicSignals = JSON.parse(fs.readFileSync(publicPath, "utf8"));

metadata.public_signal_count = publicSignals.length;
metadata.public_signal_order = Array.from(
  { length: publicSignals.length },
  (_, index) => `public_output_${index}`,
);
metadata.notes = [
  "The public signal order is positional because this fixture intentionally keeps generic snarkjs semantics only.",
  "This fixture intentionally adds no ZK Email-specific Rust-side semantics."
];

fs.writeFileSync(metadataPath, `${JSON.stringify(metadata, null, 2)}\n`);
EOF
