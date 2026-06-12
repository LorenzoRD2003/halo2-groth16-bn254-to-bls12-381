use ff::{Field, PrimeField};
use midnight_circuits::{
  field::NativeChip,
  hash::poseidon::PoseidonChip,
  instructions::{AssignmentInstructions, hash::HashCPU, hash::HashInstructions},
  field::foreign::params::{FieldEmulationParams, MultiEmulationParams},
  midnight_proofs::plonk::Error,
  types::AssignedNative,
  types::Instantiable,
};

use crate::{
  AssignedFp, Bls12HostField, ForeignField, Groth16Bn254G1Point, Groth16Bn254VerifyingKey,
  NativeField,
  bn254::Bn254FieldChip,
};

use super::poseidon_fq_params::{
  POSEIDON_FQ_ARK, POSEIDON_FQ_FULL_ROUNDS, POSEIDON_FQ_MDS, POSEIDON_FQ_PARTIAL_ROUNDS,
};

/// Stable semantic field name used for the public VK commitment component.
pub const OUTER_VK_COMMITMENT_FIELD_NAME: &str = "vk_commitment";

const VK_COMMITMENT_DOMAIN_TAG: u64 = 9_131;
const VK_COMMITMENT_SEQUENCE_TAG: u64 = 77_021;
const BLS12_COMMITMENT_PUBLIC_LIMB_BYTES: usize = 16;

fn g1_coordinates(point: Groth16Bn254G1Point) -> (ForeignField, ForeignField) {
  match point {
    Groth16Bn254G1Point::Identity => (ForeignField::ZERO, ForeignField::ZERO),
    Groth16Bn254G1Point::Affine { x, y } => (x, y),
  }
}

fn vk_coordinates(vk: &Groth16Bn254VerifyingKey) -> Vec<ForeignField> {
  let mut coordinates = Vec::with_capacity(14 + vk.ic.len() * 2);
  let alpha = g1_coordinates(vk.alpha_g1);
  coordinates.extend([alpha.0, alpha.1]);
  coordinates.extend([(vk.beta_g2.0).0, (vk.beta_g2.0).1, (vk.beta_g2.1).0, (vk.beta_g2.1).1]);
  coordinates.extend([(vk.gamma_g2.0).0, (vk.gamma_g2.0).1, (vk.gamma_g2.1).0, (vk.gamma_g2.1).1]);
  coordinates.extend([(vk.delta_g2.0).0, (vk.delta_g2.0).1, (vk.delta_g2.1).0, (vk.delta_g2.1).1]);

  for point in &vk.ic {
    let (x, y) = g1_coordinates(*point);
    coordinates.extend([x, y]);
  }

  coordinates
}

fn vk_coordinate_limbs_for_bls12(vk: &Groth16Bn254VerifyingKey) -> Vec<Bls12HostField> {
  let mut limbs = Vec::new();
  for coordinate in vk_coordinates(vk) {
    limbs.extend(AssignedFp::<Bls12HostField>::as_public_input(&coordinate));
  }
  limbs
}

fn fq(value: &str) -> ForeignField {
  ForeignField::from_str_vartime(value).expect("Poseidon BN254 Fq constant should parse")
}

fn poseidon_round_constants(round: usize) -> [ForeignField; 3] {
  let offset = round * 3;
  [fq(POSEIDON_FQ_ARK[offset]), fq(POSEIDON_FQ_ARK[offset + 1]), fq(POSEIDON_FQ_ARK[offset + 2])]
}

fn poseidon_mds() -> [[ForeignField; 3]; 3] {
  [
    [fq(POSEIDON_FQ_MDS[0][0]), fq(POSEIDON_FQ_MDS[0][1]), fq(POSEIDON_FQ_MDS[0][2])],
    [fq(POSEIDON_FQ_MDS[1][0]), fq(POSEIDON_FQ_MDS[1][1]), fq(POSEIDON_FQ_MDS[1][2])],
    [fq(POSEIDON_FQ_MDS[2][0]), fq(POSEIDON_FQ_MDS[2][1]), fq(POSEIDON_FQ_MDS[2][2])],
  ]
}

fn sbox_host(value: ForeignField) -> ForeignField {
  let square = value.square();
  let fourth = square.square();
  fourth * value
}

fn sbox_assigned<FHost>(
  chip: &Bn254FieldChip<FHost>,
  layouter: &mut impl midnight_circuits::midnight_proofs::circuit::Layouter<FHost>,
  value: &AssignedFp<FHost>,
) -> Result<AssignedFp<FHost>, Error>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  let square = chip.square(layouter, value)?;
  let fourth = chip.square(layouter, &square)?;
  chip.mul(layouter, &fourth, value)
}

fn mds_host(state: [ForeignField; 3]) -> [ForeignField; 3] {
  let mds = poseidon_mds();
  let mut next = [ForeignField::ZERO; 3];

  for row in 0..3 {
    next[row] = state[0] * mds[row][0] + state[1] * mds[row][1] + state[2] * mds[row][2];
  }

  next
}

fn mds_assigned<FHost>(
  chip: &Bn254FieldChip<FHost>,
  layouter: &mut impl midnight_circuits::midnight_proofs::circuit::Layouter<FHost>,
  state: &[AssignedFp<FHost>; 3],
) -> Result<[AssignedFp<FHost>; 3], Error>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  let mds = poseidon_mds();
  let mut next = Vec::with_capacity(3);

  for row in 0..3 {
    let t0 = chip.mul_by_constant(layouter, &state[0], mds[row][0])?;
    let t1 = chip.mul_by_constant(layouter, &state[1], mds[row][1])?;
    let t2 = chip.mul_by_constant(layouter, &state[2], mds[row][2])?;
    let sum01 = chip.add(layouter, &t0, &t1)?;
    let sum = chip.add(layouter, &sum01, &t2)?;
    next.push(sum);
  }

  Ok([next.remove(0), next.remove(0), next.remove(0)])
}

fn poseidon_compress_host(left: ForeignField, right: ForeignField) -> ForeignField {
  let mut state = [ForeignField::from(VK_COMMITMENT_DOMAIN_TAG), left, right];
  let half_full_rounds = POSEIDON_FQ_FULL_ROUNDS / 2;
  let all_rounds = POSEIDON_FQ_FULL_ROUNDS + POSEIDON_FQ_PARTIAL_ROUNDS;

  for round in 0..half_full_rounds {
    let ark = poseidon_round_constants(round);
    for index in 0..3 {
      state[index] += ark[index];
      state[index] = sbox_host(state[index]);
    }
    state = mds_host(state);
  }

  for round in half_full_rounds..half_full_rounds + POSEIDON_FQ_PARTIAL_ROUNDS {
    let ark = poseidon_round_constants(round);
    for index in 0..3 {
      state[index] += ark[index];
    }
    state[0] = sbox_host(state[0]);
    state = mds_host(state);
  }

  for round in half_full_rounds + POSEIDON_FQ_PARTIAL_ROUNDS..all_rounds {
    let ark = poseidon_round_constants(round);
    for index in 0..3 {
      state[index] += ark[index];
      state[index] = sbox_host(state[index]);
    }
    state = mds_host(state);
  }

  state[0]
}

fn poseidon_compress_assigned<FHost>(
  chip: &Bn254FieldChip<FHost>,
  layouter: &mut impl midnight_circuits::midnight_proofs::circuit::Layouter<FHost>,
  left: &AssignedFp<FHost>,
  right: &AssignedFp<FHost>,
) -> Result<AssignedFp<FHost>, Error>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  let mut state = [
    chip.assign(
      layouter,
      midnight_circuits::midnight_proofs::circuit::Value::known(ForeignField::from(
        VK_COMMITMENT_DOMAIN_TAG,
      )),
    )?,
    left.clone(),
    right.clone(),
  ];
  let half_full_rounds = POSEIDON_FQ_FULL_ROUNDS / 2;
  let all_rounds = POSEIDON_FQ_FULL_ROUNDS + POSEIDON_FQ_PARTIAL_ROUNDS;

  for round in 0..half_full_rounds {
    let ark = poseidon_round_constants(round);
    for index in 0..3 {
      state[index] = chip.add_constant(layouter, &state[index], ark[index])?;
      state[index] = sbox_assigned(chip, layouter, &state[index])?;
    }
    state = mds_assigned(chip, layouter, &state)?;
  }

  for round in half_full_rounds..half_full_rounds + POSEIDON_FQ_PARTIAL_ROUNDS {
    let ark = poseidon_round_constants(round);
    for index in 0..3 {
      state[index] = chip.add_constant(layouter, &state[index], ark[index])?;
    }
    state[0] = sbox_assigned(chip, layouter, &state[0])?;
    state = mds_assigned(chip, layouter, &state)?;
  }

  for round in half_full_rounds + POSEIDON_FQ_PARTIAL_ROUNDS..all_rounds {
    let ark = poseidon_round_constants(round);
    for index in 0..3 {
      state[index] = chip.add_constant(layouter, &state[index], ark[index])?;
      state[index] = sbox_assigned(chip, layouter, &state[index])?;
    }
    state = mds_assigned(chip, layouter, &state)?;
  }

  Ok(state[0].clone())
}

/// Computes the canonical Poseidon-based VK commitment from the normalized Groth16 VK object.
pub fn groth16_vk_commitment(vk: &Groth16Bn254VerifyingKey) -> ForeignField {
  let coordinates = vk_coordinates(vk);
  let mut state = poseidon_compress_host(
    ForeignField::from(VK_COMMITMENT_SEQUENCE_TAG),
    ForeignField::from(coordinates.len() as u64),
  );

  for coordinate in coordinates {
    state = poseidon_compress_host(state, coordinate);
  }

  poseidon_compress_host(state, ForeignField::from(VK_COMMITMENT_SEQUENCE_TAG))
}

/// Computes the canonical Poseidon-based VK commitment on the BLS12 host field.
#[must_use]
pub fn groth16_vk_commitment_bls12(vk: &Groth16Bn254VerifyingKey) -> Bls12HostField {
  let mut preimage = vec![
    Bls12HostField::from(VK_COMMITMENT_SEQUENCE_TAG),
    Bls12HostField::from(vk_coordinates(vk).len() as u64),
  ];
  preimage.extend(vk_coordinate_limbs_for_bls12(vk));
  preimage.push(Bls12HostField::from(VK_COMMITMENT_SEQUENCE_TAG));
  <PoseidonChip<Bls12HostField> as HashCPU<Bls12HostField, Bls12HostField>>::hash(&preimage)
}

/// Assigns the full normalized VK as non-native witnesses and recomputes the
/// canonical Poseidon-based commitment on the BLS12 host field.
pub fn assign_and_commit_verification_key_on_bls12_host(
  field_chip: &Bn254FieldChip<Bls12HostField>,
  native_chip: &NativeChip<Bls12HostField>,
  poseidon_chip: &PoseidonChip<Bls12HostField>,
  layouter: &mut impl midnight_circuits::midnight_proofs::circuit::Layouter<Bls12HostField>,
  vk: &Groth16Bn254VerifyingKey,
) -> Result<AssignedNative<Bls12HostField>, Error> {
  let coordinates = vk_coordinates(vk);
  let limb_width = AssignedFp::<Bls12HostField>::as_public_input(&ForeignField::ZERO).len();
  let mut preimage = Vec::with_capacity(2 + coordinates.len() * limb_width + 1);
  preimage.push(native_chip.assign_fixed(layouter, Bls12HostField::from(VK_COMMITMENT_SEQUENCE_TAG))?);
  preimage.push(native_chip.assign_fixed(layouter, Bls12HostField::from(coordinates.len() as u64))?);

  for coordinate in coordinates {
    let assigned_coordinate = field_chip.assign(
      layouter,
      midnight_circuits::midnight_proofs::circuit::Value::known(coordinate),
    )?;
    preimage.extend(assigned_coordinate.limb_values());
  }

  preimage.push(native_chip.assign_fixed(layouter, Bls12HostField::from(VK_COMMITMENT_SEQUENCE_TAG))?);
  poseidon_chip.hash(layouter, &preimage)
}

fn bls12_commitment_public_input_limbs(value: Bls12HostField) -> [NativeField; 2] {
  let repr = value.to_repr();
  let bytes = repr.as_ref();
  let mut limbs = [NativeField::ZERO; 2];

  for (index, chunk) in bytes.chunks(BLS12_COMMITMENT_PUBLIC_LIMB_BYTES).enumerate().take(2) {
    let mut acc = NativeField::ZERO;
    let radix = NativeField::from(256_u64);
    for byte in chunk.iter().rev() {
      acc = acc * radix + NativeField::from(u64::from(*byte));
    }
    limbs[index] = acc;
  }

  limbs
}

/// Flattens one semantic VK commitment field element into host-lane public inputs.
pub fn groth16_vk_commitment_public_inputs(value: ForeignField) -> Vec<NativeField> {
  AssignedFp::<NativeField>::as_public_input(&value)
}

/// Flattens one semantic BLS12 VK commitment value into compatibility-lane public inputs.
#[must_use]
pub fn groth16_vk_commitment_bls12_public_inputs(value: Bls12HostField) -> Vec<NativeField> {
  bls12_commitment_public_input_limbs(value).to_vec()
}

/// Returns the flattened public-input names used to expose one semantic VK commitment.
pub fn groth16_vk_commitment_public_input_names(field_name: &str) -> Vec<String> {
  groth16_vk_commitment_public_inputs(ForeignField::ZERO)
    .iter()
    .enumerate()
    .map(|(index, _)| format!("{field_name}_limb_{index}"))
    .collect()
}

/// Returns the flattened public-input names used to expose one semantic BLS12 VK commitment.
#[must_use]
pub fn groth16_vk_commitment_bls12_public_input_names(field_name: &str) -> Vec<String> {
  groth16_vk_commitment_bls12_public_inputs(Bls12HostField::ZERO)
    .iter()
    .enumerate()
    .map(|(index, _)| format!("{field_name}_limb_{index}"))
    .collect()
}

/// Assigns the full normalized VK as non-native witnesses and recomputes the canonical Poseidon-based commitment.
pub fn assign_and_commit_verification_key_on_host<FHost>(
  chip: &Bn254FieldChip<FHost>,
  layouter: &mut impl midnight_circuits::midnight_proofs::circuit::Layouter<FHost>,
  vk: &Groth16Bn254VerifyingKey,
) -> Result<AssignedFp<FHost>, Error>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  let coordinates = vk_coordinates(vk);
  let sequence_tag = chip.assign(
    layouter,
    midnight_circuits::midnight_proofs::circuit::Value::known(ForeignField::from(
      VK_COMMITMENT_SEQUENCE_TAG,
    )),
  )?;
  let coordinate_len = chip.assign(
    layouter,
    midnight_circuits::midnight_proofs::circuit::Value::known(ForeignField::from(
      coordinates.len() as u64,
    )),
  )?;
  let mut state = poseidon_compress_assigned(chip, layouter, &sequence_tag, &coordinate_len)?;

  for coordinate in coordinates {
    let assigned_coordinate = chip
      .assign(layouter, midnight_circuits::midnight_proofs::circuit::Value::known(coordinate))?;
    state = poseidon_compress_assigned(chip, layouter, &state, &assigned_coordinate)?;
  }

  let final_tag = chip.assign(
    layouter,
    midnight_circuits::midnight_proofs::circuit::Value::known(ForeignField::from(
      VK_COMMITMENT_SEQUENCE_TAG,
    )),
  )?;
  poseidon_compress_assigned(chip, layouter, &state, &final_tag)
}
