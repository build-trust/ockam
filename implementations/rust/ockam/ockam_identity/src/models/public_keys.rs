use ockam_core::Result;

use core::ops::Deref;
use minicbor::bytes::ByteArray;
use minicbor::encode::Write;
use minicbor::{Decode, Decoder, Encode, Encoder};

/// Ed25519 Public Key
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ed25519PublicKey(pub [u8; 32]);

/// X25519 Public Key
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct X25519PublicKey(pub [u8; 32]);

/// P256 Public Key
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct P256ECDSAPublicKey(pub [u8; 65]);

impl<C> Encode<C> for Ed25519PublicKey {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        ByteArray::from(self.0).encode(e, ctx)
    }
}

impl<'b, C> Decode<'b, C> for Ed25519PublicKey {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let data = ByteArray::<32>::decode(d, ctx)?;

        Ok(Self(*data.deref()))
    }
}

impl<C> Encode<C> for X25519PublicKey {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        ByteArray::from(self.0).encode(e, ctx)
    }
}

impl<'b, C> Decode<'b, C> for X25519PublicKey {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let data = ByteArray::<32>::decode(d, ctx)?;

        Ok(Self(*data.deref()))
    }
}

impl<C> Encode<C> for P256ECDSAPublicKey {
    fn encode<W: Write>(
        &self,
        e: &mut Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        ByteArray::from(self.0).encode(e, ctx)
    }
}

impl<'b, C> Decode<'b, C> for P256ECDSAPublicKey {
    fn decode(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let data = ByteArray::<65>::decode(d, ctx)?;

        Ok(Self(*data.deref()))
    }
}
