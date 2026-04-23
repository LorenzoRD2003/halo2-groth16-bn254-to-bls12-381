use ark_bn254::{Fq12 as ArkFq12, G1Affine as ArkG1Affine};
use ark_ec::AffineRepr;
use ark_ff::Field as ArkField;
use ff::Field;
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner},
  plonk::{Circuit, ConstraintSystem, Error},
};
use midnight_proofs::dev::MockProver;

use super::{
  Groth16Bn254VerifierCircuit, Groth16Bn254VerifyingKey, Groth16IcAccumulatorCircuit, NativeField,
  fixtures::typed::{
    proof as fixture_proof, public_inputs as fixture_public_inputs, verifying_key as fixture_vk,
  },
  groth16_accumulate_ic_legacy,
  reference::{ark_to_midnight_g1, host_pairing_product, host_public_input_accumulator},
};
use crate::bn254::{Bn254G1Chip, Bn254G1Config, ForeignCurve};

#[derive(Clone, Debug)]
struct Groth16IcAccumulatorLegacyComparisonCircuit {
  vk: Groth16Bn254VerifyingKey,
  public_inputs: Vec<NativeField>,
  expected: ForeignCurve,
}

impl Groth16IcAccumulatorLegacyComparisonCircuit {
  fn new(
    vk: Groth16Bn254VerifyingKey,
    public_inputs: Vec<NativeField>,
    expected: ForeignCurve,
  ) -> Self {
    Self { vk, public_inputs, expected }
  }
}

impl Circuit<NativeField> for Groth16IcAccumulatorLegacyComparisonCircuit {
  type Config = Bn254G1Config;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      vk: self.vk.clone(),
      public_inputs: vec![NativeField::ZERO; self.public_inputs.len()],
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254G1Config::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let g1_chip = Bn254G1Chip::new(&config);
    let optimized =
      super::groth16_accumulate_ic(&g1_chip, &mut layouter, &self.vk, &self.public_inputs)
        .map_err(|error| match error {
          super::Groth16VerifierError::Circuit(inner) => inner,
          _ => Error::Synthesis(error.to_string()),
        })?;
    let legacy =
      groth16_accumulate_ic_legacy(&g1_chip, &mut layouter, &self.vk, &self.public_inputs)
        .map_err(|error| match error {
          super::Groth16VerifierError::Circuit(inner) => inner,
          _ => Error::Synthesis(error.to_string()),
        })?;

    g1_chip.assert_equal_to_fixed(&mut layouter, &optimized, self.expected)?;
    g1_chip.assert_equal_to_fixed(&mut layouter, &legacy, self.expected)?;
    g1_chip.load(&mut layouter)
  }
}

fn assert_satisfied<C: midnight_proofs::plonk::Circuit<NativeField>>(k: u32, circuit: &C) {
  let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect("MockProver should build");
  assert_eq!(prover.verify(), Ok(()));
}

#[test]
fn groth16_ic_accumulator_matches_arkworks_reference() {
  let vk = fixture_vk();
  let public_inputs = fixture_public_inputs();
  let expected = ark_to_midnight_g1(host_public_input_accumulator(&vk, &public_inputs));

  assert_satisfied(14, &Groth16IcAccumulatorCircuit::new(vk, public_inputs, expected));
}

#[test]
fn groth16_ic_accumulator_matches_legacy_path() {
  let vk = fixture_vk();
  let public_inputs = fixture_public_inputs();
  let expected = ark_to_midnight_g1(host_public_input_accumulator(&vk, &public_inputs));

  assert_satisfied(
    14,
    &Groth16IcAccumulatorLegacyComparisonCircuit::new(vk, public_inputs, expected),
  );
}

#[test]
fn groth16_ic_accumulator_rejects_public_input_length_mismatch() {
  let circuit = Groth16IcAccumulatorCircuit::new(
    fixture_vk(),
    Vec::new(),
    ark_to_midnight_g1(ArkG1Affine::generator()),
  );

  let result = MockProver::run(14, &circuit, vec![vec![], vec![]]);
  assert!(result.is_err(), "length mismatch should fail during synthesis");
}

#[test]
fn groth16_pairing_product_encoding_matches_arkworks_verifier_relation() {
  let product = host_pairing_product(&fixture_vk(), &fixture_proof(), &fixture_public_inputs());

  assert_eq!(product, ArkFq12::ONE);
}

// These full-circuit MockProver checks are kept as explicit slow integration
// tests because the Week 5 pairing-backed verifier circuit is still too heavy
// for the default local lane. Always-run end-to-end acceptance/rejection lives
// in `wrapper-tests`, while these remain the highest-fidelity circuit stress
// checks for deliberate slow-lane execution.
#[test]
#[ignore = "slow pairing-core"]
fn slow_mockprover_valid_real_fixture_is_accepted_end_to_end() {
  assert_satisfied(
    22,
    &Groth16Bn254VerifierCircuit::new(fixture_vk(), fixture_proof(), fixture_public_inputs(), true),
  );
}

#[test]
#[ignore = "slow pairing-core"]
fn slow_mockprover_invalid_public_input_mutation_is_rejected_end_to_end() {
  let mut public_inputs = fixture_public_inputs();
  public_inputs[0] = NativeField::from(34_u64);

  assert_satisfied(
    22,
    &Groth16Bn254VerifierCircuit::new(fixture_vk(), fixture_proof(), public_inputs, false),
  );
}
