//! Inner-verifier semantics owned by the canonical outer wrapper circuit.

use ff::{Field, PrimeField};
use midnight_circuits::field::foreign::params::{FieldEmulationParams, MultiEmulationParams};
use midnight_circuits::midnight_proofs::{
  circuit::Layouter,
  plonk::{Column, Error, Instance},
};

use crate::{
  Bn254BoolChip, Bn254BoolConfig, assign_and_commit_verification_key_on_host,
  bn254::{Bn254FieldChip, Bn254FieldConfig},
  groth16::groth16_verify_on_host,
  outer::OuterVerificationKeyCommitmentValue,
};

use super::{OuterHostField, OuterWrapperCircuitInput};

/// BN254 Groth16 verifier semantics configured as non-native logic inside the outer circuit.
#[derive(Clone, Debug)]
pub struct Bn254InnerVerifierConfig<FHost = OuterHostField>
where
  FHost: PrimeField,
  MultiEmulationParams: FieldEmulationParams<FHost, crate::ForeignField>,
{
  field: Bn254FieldConfig<FHost>,
  bools: Bn254BoolConfig<FHost>,
}

impl<FHost> Bn254InnerVerifierConfig<FHost>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, crate::ForeignField>,
{
  pub(crate) fn field_config(&self) -> &Bn254FieldConfig<FHost> {
    &self.field
  }

  /// Configures the BN254 verifier semantics against one chosen host-lane
  /// instance-column layout.
  #[must_use]
  pub fn configure(
    meta: &mut midnight_circuits::midnight_proofs::plonk::ConstraintSystem<FHost>,
    instance_columns: &[Column<Instance>; 2],
  ) -> Self {
    Self {
      field: Bn254FieldConfig::configure_with_instances(meta, instance_columns),
      bools: Bn254BoolConfig::configure_with_instances(meta, instance_columns),
    }
  }

  /// Synthesizes the BN254 Groth16 verifier semantics over the current host lane.
  ///
  /// # Errors
  ///
  /// Returns any verifier, assignment, or gate-loading failure raised while
  /// proving the current BN254 inner verifier semantics.
  pub fn synthesize(
    &self,
    layouter: &mut impl Layouter<FHost>,
    input: &OuterWrapperCircuitInput,
  ) -> Result<(), Error> {
    let field_chip = Bn254FieldChip::new(&self.field);
    let bool_chip = Bn254BoolChip::new(&self.bools);
    if let OuterVerificationKeyCommitmentValue::Bn254(expected_commitment) =
      input.outer_statement.vk_commitment.value
    {
      let vk_commitment = assign_and_commit_verification_key_on_host(
        &field_chip,
        layouter,
        &input.inner_verification_key,
      )?;
      field_chip.assert_equal_to_fixed(layouter, &vk_commitment, expected_commitment)?;
    }
    let result = groth16_verify_on_host(
      &field_chip,
      &bool_chip,
      layouter,
      &input.inner_verification_key,
      &input.inner_proof,
      &input.inner_public_inputs,
    )
    .map_err(|error| match error {
      crate::Groth16VerifierError::Circuit(inner) => inner,
      _ => Error::Synthesis(error.to_string()),
    })?;

    bool_chip.assert_equal_to_fixed(layouter, &result, true)?;
    field_chip.load(layouter)?;
    bool_chip.load(layouter)
  }
}
