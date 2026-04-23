use ark_bn254::{Fq as ArkFq, Fr as ArkFr, G1Affine as ArkG1Affine};
use ark_ec::AffineRepr;
use ark_ff::{BigInteger, PrimeField as ArkPrimeField};
use ff::PrimeField;
use halo2curves::group::Group;
use midnight_curves::{CurveAffine, bn256::G1Affine};

use crate::bn254::{ForeignCurve, ForeignField, NativeField};

pub(crate) fn midnight_to_ark_fq(value: ForeignField) -> ArkFq {
  ArkFq::from_le_bytes_mod_order(value.to_repr().as_ref())
}

pub(crate) fn midnight_to_ark_fr(value: NativeField) -> ArkFr {
  ArkFr::from_le_bytes_mod_order(value.to_repr().as_ref())
}

pub(crate) fn ark_to_midnight_fq(value: ArkFq) -> ForeignField {
  let bytes = value.into_bigint().to_bytes_le();
  let mut repr = <ForeignField as PrimeField>::Repr::default();
  let repr_bytes = repr.as_mut();
  let copy_len = bytes.len().min(repr_bytes.len());
  repr_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);

  ForeignField::from_repr_vartime(repr)
    .expect("arkworks bn254 fq value should fit midnight bn254 fq")
}

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
