use crate::models::{Ed25519Signature, P256ECDSASignature};
use core::ops::Deref;
use minicbor::bytes::ByteArray;
use minicbor::encode::Write;
use minicbor::{Decode, Decoder, Encode, Encoder};
use ockam_vault::Signature;

impl<C> Encode<C> for Ed25519Signature {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        ByteArray::from(self.0).encode(e, ctx)
    }
}

impl<'b, C> Decode<'b, C> for Ed25519Signature {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let data = ByteArray::<64>::decode(d, ctx)?;

        Ok(Self(*data.deref()))
    }
}

impl<C> Encode<C> for P256ECDSASignature {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        ByteArray::from(self.0).encode(e, ctx)
    }
}

impl<'b, C> Decode<'b, C> for P256ECDSASignature {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let data = ByteArray::<64>::decode(d, ctx)?;

        Ok(Self(*data.deref()))
    }
}

impl From<Ed25519Signature> for Signature {
    fn from(value: Ed25519Signature) -> Self {
        Self::new(value.0.to_vec())
    }
}

impl From<P256ECDSASignature> for Signature {
    fn from(value: P256ECDSASignature) -> Self {
        Self::new(value.0.to_vec())
    }
}
