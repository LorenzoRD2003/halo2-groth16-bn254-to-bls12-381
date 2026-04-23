# Circom/snarkjs Groth16 BN254 Fixture

This fixture is the Week 5 end-to-end regression input for the first narrow
Groth16 BN254 verifier slice.

Source shape:

- Circuit: `multiplier2.circom`
- Input: `a = 3`, `b = 11`
- Public output / public input to the verifier: `33`
- Curve label emitted by snarkjs: `bn128` (the BN254 family used by the repo)

Generated with:

- `circom` 2.2.3
- `npx -y snarkjs` 0.7.6

Committed artifacts:

- `verification_key.json`
- `proof.json`
- `public.json`

The fixture is intentionally tiny so regression failures are easier to inspect.
The invalid verifier test mutates the parsed public input from `33` to `34`
while keeping the proof and VK structurally valid.

Test layering around this fixture:

- always-run end-to-end acceptance/rejection lives in `wrapper-tests`
- full-circuit MockProver variants remain ignored slow tests in `wrapper-circuits`
