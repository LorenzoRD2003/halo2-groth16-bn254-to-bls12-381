use ark_bn254::{Fq as ArkFq, Fr as ArkFr};
use ark_ff::PrimeField as ArkPrimeField;
use ff::PrimeField;

use crate::bn254::{ForeignField, NativeField};

#[cfg(test)]
use {
  crate::bn254::ForeignCurve,
  ark_bn254::G1Affine as ArkG1Affine,
  ark_ec::AffineRepr,
  ark_ff::BigInteger,
  halo2curves::group::Group,
  midnight_curves::{CurveAffine, bn256::G1Affine},
};

pub(crate) fn midnight_to_ark_fq(value: ForeignField) -> ArkFq {
  ArkFq::from_le_bytes_mod_order(value.to_repr().as_ref())
}

pub(crate) fn midnight_to_ark_fr(value: NativeField) -> ArkFr {
  ArkFr::from_le_bytes_mod_order(value.to_repr().as_ref())
}

#[cfg(test)]
pub(crate) fn ark_to_midnight_fq(value: ArkFq) -> ForeignField {
  let bytes = value.into_bigint().to_bytes_le();
  let mut repr = <ForeignField as PrimeField>::Repr::default();
  let repr_bytes = repr.as_mut();
  let copy_len = bytes.len().min(repr_bytes.len());
  repr_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);

  ForeignField::from_repr_vartime(repr)
    .expect("arkworks bn254 fq value should fit midnight bn254 fq")
}

#[cfg(test)]
pub(crate) fn ark_to_midnight_g1(point: ArkG1Affine) -> ForeignCurve {
  if point.is_zero() {
    return ForeignCurve::identity();
  }

  let affine = Option::<G1Affine>::from(G1Affine::from_xy(
    ark_to_midnight_fq(point.x),
    ark_to_midnight_fq(point.y),
  ))
  .expect("arkworks point should map to a valid midnight bn254 point");

  affine.into()
}
