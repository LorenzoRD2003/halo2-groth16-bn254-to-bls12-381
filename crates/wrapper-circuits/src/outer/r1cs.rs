use thiserror::Error;
use wrapper_core::WrapperError;

use ark_bn254::{
  Bn254 as ArkBn254, Fq as ArkFq, Fq2 as ArkFq2, Fq12 as ArkFq12, G1Affine as ArkG1Affine,
  G2Affine as ArkG2Affine,
};
use ark_ec::{AffineRepr, CurveGroup, pairing::Pairing};
use ark_ff::{Field as ArkField, PrimeField as ArkPrimeField};
use ff::Field;
use ff::PrimeField;

use super::OuterWrapperCircuitInput;
use crate::groth16::groth16_public_input_accumulator_constant;
use crate::{ForeignField, Groth16Bn254Proof, Groth16Bn254VerifyingKey};
use crate::{
  Groth16Bn254G1Point, Halo2CellLinearCombination, Halo2CellRef, Halo2Phase1R1csLowering,
  Halo2PublicInputRef, Halo2R1csMetadata, NativeField, R1csCircuit, VariableId,
};

/// Deterministic canonical R1CS lowering for the explicit outer-statement exposure slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterStatementExposureR1cs {
  /// Canonical metadata boundary for the exposed outer statement cells.
  pub metadata: Halo2R1csMetadata,
  /// Canonical R1CS for the exposed outer statement slice.
  pub circuit: R1csCircuit,
}

/// Deterministic extraction of the verifier-side IC accumulator slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterGroth16IcAccumulatorSlice {
  /// Ordered public inputs consumed by the verifier-side accumulator.
  pub public_inputs: Vec<NativeField>,
  /// Ordered IC table points from the inner verification key.
  pub ic_points: Vec<Groth16Bn254G1Point>,
  /// Canonical public-input variables feeding the accumulator schedule.
  pub public_input_variables: Vec<VariableId>,
  /// Canonical witness variables representing the ordered accumulator scalar schedule.
  pub scheduled_scalar_variables: Vec<VariableId>,
  /// Host-side reference accumulator `vk_x`.
  pub expected_accumulator: Groth16Bn254G1Point,
  /// Canonical R1CS slice for the ordered scalar schedule consumed by the accumulator.
  pub circuit: R1csCircuit,
}

/// Deterministic extraction of the final verifier-result assertion slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterVerifierResultAssertionSlice {
  /// Canonical witness variable representing the verifier result.
  pub result_variable: VariableId,
  /// Fixed boolean value required by the outer wrapper circuit.
  pub expected_result: bool,
  /// Human-readable statement of what is being asserted.
  pub assertion_rule: &'static str,
  /// Canonical R1CS slice asserting `result = 1`.
  pub circuit: R1csCircuit,
}

/// Deterministic extraction of the Groth16 pairing-product check slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterGroth16PairingProductCheckSlice {
  /// Inner proof consumed by the pairing-product relation.
  pub proof: Groth16Bn254Proof,
  /// Inner verification key consumed by the pairing-product relation.
  pub verification_key: Groth16Bn254VerifyingKey,
  /// Ordered public inputs consumed by the verifier relation.
  pub public_inputs: Vec<NativeField>,
  /// Host-side Groth16 pairing product.
  pub expected_pairing_product: ArkFq12,
  /// Whether the host-side pairing product equals the target-group identity.
  pub expected_is_identity: bool,
}

/// Slice kinds in the incremental canonical R1CS lowering of the outer wrapper.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OuterCanonicalR1csSliceKind {
  /// Advice-to-instance exposure of the outer statement.
  OuterStatementExposure,
  /// Verifier-side G1 IC accumulator over public inputs.
  Groth16IcAccumulator,
  /// Groth16 pairing-product check reduced to one boolean result.
  Groth16PairingProductCheck,
  /// Final assertion that the verifier result equals true.
  VerifierResultAssertion,
}

/// Status of one outer-wrapper canonical R1CS lowering slice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OuterCanonicalR1csSliceStatus {
  /// The slice is already lowered into the current canonical R1CS circuit.
  Lowered,
  /// The slice has deterministic extracted inputs/reference data, but still
  /// lacks full canonical R1CS lowering.
  Prepared {
    /// Current blocker for promoting the slice to fully lowered.
    reason: &'static str,
  },
  /// The slice is not lowered yet and carries one explicit blocker note.
  Pending {
    /// Current blocker for this slice.
    reason: &'static str,
  },
}

/// One slice-level report entry for the outer-wrapper canonical R1CS lowering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterCanonicalR1csSliceReport {
  /// Slice kind being reported.
  pub kind: OuterCanonicalR1csSliceKind,
  /// Current lowering status for this slice.
  pub status: OuterCanonicalR1csSliceStatus,
}

/// Incremental lowering report for the canonical outer wrapper R1CS path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OuterCanonicalR1csLoweringReport {
  /// The currently lowered outer-statement exposure slice.
  pub statement_exposure: OuterStatementExposureR1cs,
  /// Deterministic extraction of the verifier-side IC accumulator slice.
  pub ic_accumulator: OuterGroth16IcAccumulatorSlice,
  /// Deterministic extraction of the pairing-product check slice.
  pub pairing_product_check: OuterGroth16PairingProductCheckSlice,
  /// Deterministic extraction of the final verifier-result assertion slice.
  pub verifier_result_assertion: OuterVerifierResultAssertionSlice,
  /// Slice-by-slice status for the outer wrapper lowering path.
  pub slices: Vec<OuterCanonicalR1csSliceReport>,
}

/// Errors raised while lowering the canonical outer circuit to canonical R1CS.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum OuterCanonicalR1csLoweringError {
  /// The outer circuit input is not valid enough to attempt canonical lowering.
  #[error("outer circuit input is not ready for canonical R1CS lowering: {reason}")]
  InvalidInput {
    /// Human-readable reason for the rejected input.
    reason: String,
  },
  /// The verifier body still has no canonical lowering path.
  #[error(
    "outer verifier body still has no canonical R1CS lowering; pending slices: {pending_slices:?}"
  )]
  UnsupportedVerifierBodyLowering {
    /// Verifier-body slices that still block the full canonical lowering.
    pending_slices: Vec<OuterCanonicalR1csSliceKind>,
  },
}

impl From<WrapperError> for OuterCanonicalR1csLoweringError {
  fn from(error: WrapperError) -> Self {
    Self::InvalidInput { reason: error.to_string() }
  }
}

/// Builds canonical metadata plus R1CS for the explicit outer-statement exposure slice.
///
/// This covers only the advice-to-instance exposure of the outer public
/// statement. It does not include the inner verifier body.
///
/// # Errors
///
/// Returns an error if the outer wrapper input is not valid.
pub fn build_outer_statement_exposure_r1cs(
  input: &OuterWrapperCircuitInput,
) -> Result<OuterStatementExposureR1cs, OuterCanonicalR1csLoweringError> {
  input.validate()?;

  let mut cells = Vec::with_capacity(input.outer_statement.public_inputs.len() * 2);
  let mut equality_edges = Vec::with_capacity(input.outer_statement.public_inputs.len());
  let mut public_inputs = Vec::with_capacity(input.outer_statement.public_inputs.len());

  for public_index in 0..input.outer_statement.public_inputs.len() {
    let advice_cell = Halo2CellRef::Advice { column: 0, row: public_index };
    let instance_cell = Halo2CellRef::Instance { column: 0, row: public_index };

    cells.push(advice_cell);
    cells.push(instance_cell);
    equality_edges.push(crate::EqualityEdge::new(advice_cell, instance_cell));
    public_inputs.push(Halo2PublicInputRef { cell: instance_cell, public_index });
  }

  let metadata = Halo2R1csMetadata { cells, equality_edges, public_inputs };
  let lowering = Halo2Phase1R1csLowering::from_metadata(&metadata)
    .map_err(|error| OuterCanonicalR1csLoweringError::InvalidInput { reason: error.to_string() })?;

  Ok(OuterStatementExposureR1cs { metadata, circuit: lowering.build() })
}

/// Builds the deterministic verifier-side IC accumulator slice.
///
/// # Errors
///
/// Returns an error if the outer wrapper input is not valid.
pub fn build_outer_groth16_ic_accumulator_slice(
  input: &OuterWrapperCircuitInput,
) -> Result<OuterGroth16IcAccumulatorSlice, OuterCanonicalR1csLoweringError> {
  input.validate()?;

  let mut cells = Vec::with_capacity(input.inner_public_inputs.len() * 2);
  let mut public_inputs = Vec::with_capacity(input.inner_public_inputs.len());
  let mut public_cells = Vec::with_capacity(input.inner_public_inputs.len());
  let mut scheduled_cells = Vec::with_capacity(input.inner_public_inputs.len());

  for public_index in 0..input.inner_public_inputs.len() {
    let public_cell = Halo2CellRef::Instance { column: 0, row: public_index };
    let scheduled_cell = Halo2CellRef::Advice { column: 1, row: public_index };
    cells.push(public_cell);
    cells.push(scheduled_cell);
    public_cells.push(public_cell);
    scheduled_cells.push(scheduled_cell);
    public_inputs.push(Halo2PublicInputRef { cell: public_cell, public_index });
  }

  let metadata = Halo2R1csMetadata { cells, equality_edges: Vec::new(), public_inputs };
  let mut lowering = Halo2Phase1R1csLowering::from_metadata(&metadata)
    .map_err(|error| OuterCanonicalR1csLoweringError::InvalidInput { reason: error.to_string() })?;

  let public_input_variables = lowering.public_variables().to_vec();
  let scheduled_scalar_variables = scheduled_cells
    .iter()
    .map(|cell| lowering.variable_for_cell(*cell))
    .collect::<Result<Vec<_>, _>>()
    .map_err(|error| OuterCanonicalR1csLoweringError::InvalidInput { reason: error.to_string() })?;

  for (scheduled_cell, public_cell) in scheduled_cells.iter().zip(&public_cells) {
    lowering
      .add_linear_gate(
        &Halo2CellLinearCombination::from_cell(*scheduled_cell),
        &Halo2CellLinearCombination::from_cell(*public_cell),
      )
      .map_err(|error| OuterCanonicalR1csLoweringError::InvalidInput {
        reason: error.to_string(),
      })?;
  }

  Ok(OuterGroth16IcAccumulatorSlice {
    public_inputs: input.inner_public_inputs.clone(),
    ic_points: input.inner_verification_key.ic.clone(),
    public_input_variables,
    scheduled_scalar_variables,
    expected_accumulator: groth16_public_input_accumulator_constant(
      &input.inner_verification_key,
      &input.inner_public_inputs,
    ),
    circuit: lowering.build(),
  })
}

/// Builds the deterministic final verifier-result assertion slice.
///
/// # Errors
///
/// Returns an error if the outer wrapper input is not valid.
pub fn build_outer_verifier_result_assertion_slice(
  input: &OuterWrapperCircuitInput,
) -> Result<OuterVerifierResultAssertionSlice, OuterCanonicalR1csLoweringError> {
  input.validate()?;

  let result_cell = Halo2CellRef::Advice { column: 2, row: 0 };
  let metadata = Halo2R1csMetadata {
    cells: vec![result_cell],
    equality_edges: Vec::new(),
    public_inputs: Vec::new(),
  };
  let mut lowering = Halo2Phase1R1csLowering::from_metadata(&metadata)
    .map_err(|error| OuterCanonicalR1csLoweringError::InvalidInput { reason: error.to_string() })?;
  let result_variable = lowering
    .variable_for_cell(result_cell)
    .map_err(|error| OuterCanonicalR1csLoweringError::InvalidInput { reason: error.to_string() })?;
  lowering
    .add_linear_constant_gate(&Halo2CellLinearCombination::from_cell(result_cell), NativeField::ONE)
    .map_err(|error| OuterCanonicalR1csLoweringError::InvalidInput { reason: error.to_string() })?;

  Ok(OuterVerifierResultAssertionSlice {
    result_variable,
    expected_result: true,
    assertion_rule: "the outer wrapper circuit asserts that the inner verifier result equals true",
    circuit: lowering.build(),
  })
}

/// Builds the deterministic Groth16 pairing-product check slice.
///
/// # Errors
///
/// Returns an error if the outer wrapper input is not valid.
pub fn build_outer_groth16_pairing_product_check_slice(
  input: &OuterWrapperCircuitInput,
) -> Result<OuterGroth16PairingProductCheckSlice, OuterCanonicalR1csLoweringError> {
  input.validate()?;

  let vk_x =
    ark_host_public_input_accumulator(&input.inner_verification_key, &input.inner_public_inputs);
  let product = [
    (groth16_g1_to_ark(input.inner_proof.a), groth16_g2_to_ark(input.inner_proof.b)),
    (
      (-groth16_g1_to_ark(input.inner_verification_key.alpha_g1).into_group()).into_affine(),
      groth16_g2_to_ark(input.inner_verification_key.beta_g2),
    ),
    ((-vk_x.into_group()).into_affine(), groth16_g2_to_ark(input.inner_verification_key.gamma_g2)),
    (
      (-groth16_g1_to_ark(input.inner_proof.c).into_group()).into_affine(),
      groth16_g2_to_ark(input.inner_verification_key.delta_g2),
    ),
  ]
  .into_iter()
  .fold(ArkFq12::ONE, |acc, (g1, g2)| acc * ArkBn254::pairing(g1, g2).0);

  Ok(OuterGroth16PairingProductCheckSlice {
    proof: input.inner_proof.clone(),
    verification_key: input.inner_verification_key.clone(),
    public_inputs: input.inner_public_inputs.clone(),
    expected_pairing_product: product,
    expected_is_identity: product == ArkFq12::ONE,
  })
}

/// Inspects the current incremental canonical R1CS lowering state for the outer wrapper.
///
/// # Errors
///
/// Returns an error if the outer wrapper input is not valid enough to begin the
/// canonical lowering process.
pub fn inspect_outer_wrapper_canonical_r1cs(
  input: &OuterWrapperCircuitInput,
) -> Result<OuterCanonicalR1csLoweringReport, OuterCanonicalR1csLoweringError> {
  let statement_exposure = build_outer_statement_exposure_r1cs(input)?;
  let ic_accumulator = build_outer_groth16_ic_accumulator_slice(input)?;
  let pairing_product_check = build_outer_groth16_pairing_product_check_slice(input)?;
  let verifier_result_assertion = build_outer_verifier_result_assertion_slice(input)?;

  Ok(OuterCanonicalR1csLoweringReport {
    statement_exposure,
    ic_accumulator,
    pairing_product_check,
    verifier_result_assertion,
    slices: vec![
      OuterCanonicalR1csSliceReport {
        kind: OuterCanonicalR1csSliceKind::OuterStatementExposure,
        status: OuterCanonicalR1csSliceStatus::Lowered,
      },
      OuterCanonicalR1csSliceReport {
        kind: OuterCanonicalR1csSliceKind::Groth16IcAccumulator,
        status: OuterCanonicalR1csSliceStatus::Lowered,
      },
      OuterCanonicalR1csSliceReport {
        kind: OuterCanonicalR1csSliceKind::Groth16PairingProductCheck,
        status: OuterCanonicalR1csSliceStatus::Prepared {
          reason: "deterministic proof/VK/public-input extraction plus host-side pairing product exist, but canonical R1CS lowering of the BN254 pairing core is still pending",
        },
      },
      OuterCanonicalR1csSliceReport {
        kind: OuterCanonicalR1csSliceKind::VerifierResultAssertion,
        status: OuterCanonicalR1csSliceStatus::Lowered,
      },
    ],
  })
}

/// Attempts to lower the full canonical outer wrapper circuit into canonical R1CS.
///
/// # Errors
///
/// Returns an error until the verifier body has a deterministic canonical
/// lowering path.
pub fn build_outer_wrapper_canonical_r1cs(
  input: &OuterWrapperCircuitInput,
) -> Result<R1csCircuit, OuterCanonicalR1csLoweringError> {
  let report = inspect_outer_wrapper_canonical_r1cs(input)?;
  let pending_slices = report
    .slices
    .iter()
    .filter_map(|slice| {
      (!matches!(slice.status, OuterCanonicalR1csSliceStatus::Lowered)).then_some(slice.kind)
    })
    .collect::<Vec<_>>();

  if pending_slices.is_empty() {
    return Ok(report.statement_exposure.circuit);
  }

  Err(OuterCanonicalR1csLoweringError::UnsupportedVerifierBodyLowering { pending_slices })
}

fn midnight_to_ark_fq(value: ForeignField) -> ArkFq {
  ArkFq::from_le_bytes_mod_order(value.to_repr().as_ref())
}

fn midnight_to_ark_fr(value: NativeField) -> ark_bn254::Fr {
  ark_bn254::Fr::from_le_bytes_mod_order(value.to_repr().as_ref())
}

fn groth16_g1_to_ark(point: Groth16Bn254G1Point) -> ArkG1Affine {
  match point {
    Groth16Bn254G1Point::Identity => ArkG1Affine::identity(),
    Groth16Bn254G1Point::Affine { x, y } => {
      ArkG1Affine::new_unchecked(midnight_to_ark_fq(x), midnight_to_ark_fq(y))
    }
  }
}

fn groth16_g2_to_ark(
  point: ((ForeignField, ForeignField), (ForeignField, ForeignField)),
) -> ArkG2Affine {
  ArkG2Affine::new_unchecked(
    ArkFq2::new(midnight_to_ark_fq((point.0).0), midnight_to_ark_fq((point.0).1)),
    ArkFq2::new(midnight_to_ark_fq((point.1).0), midnight_to_ark_fq((point.1).1)),
  )
}

fn ark_host_public_input_accumulator(
  vk: &Groth16Bn254VerifyingKey,
  public_inputs: &[NativeField],
) -> ArkG1Affine {
  let mut accumulator = groth16_g1_to_ark(vk.ic[0]).into_group();

  for (scalar, ic_point) in public_inputs.iter().zip(vk.ic.iter().skip(1)) {
    accumulator +=
      groth16_g1_to_ark(*ic_point).mul_bigint(midnight_to_ark_fr(*scalar).into_bigint());
  }

  accumulator.into_affine()
}
