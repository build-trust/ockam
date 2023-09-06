use crate::models::{Ed25519PublicKey, P256ECDSAPublicKey, X25519PublicKey};
use crate::IdentityError;

use ockam_core::{Error, Result};
use ockam_vault::{PublicKey, SecretType};

use core::ops::Deref;
use minicbor::bytes::ByteArray;
use minicbor::encode::Write;
use minicbor::{Decode, Decoder, Encode, Encoder};

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

impl From<Ed25519PublicKey> for PublicKey {
    fn from(value: Ed25519PublicKey) -> Self {
        Self::new(value.0.to_vec(), SecretType::Ed25519)
    }
}

impl From<X25519PublicKey> for PublicKey {
    fn from(value: X25519PublicKey) -> Self {
        Self::new(value.0.to_vec(), SecretType::X25519)
    }
}

impl From<P256ECDSAPublicKey> for PublicKey {
    fn from(value: P256ECDSAPublicKey) -> Self {
        Self::new(value.0.to_vec(), SecretType::NistP256)
    }
}

impl TryFrom<PublicKey> for Ed25519PublicKey {
    type Error = Error;

    fn try_from(value: PublicKey) -> Result<Self> {
        match value.stype() {
            SecretType::Ed25519 => {
                let data = value
                    .data()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidKeyData)?;
                Ok(Self(data))
            }
            _ => Err(IdentityError::InvalidKeyType.into()),
        }
    }
}

impl TryFrom<PublicKey> for X25519PublicKey {
    type Error = Error;

    fn try_from(value: PublicKey) -> Result<Self> {
        match value.stype() {
            SecretType::X25519 => {
                let data = value
                    .data()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidKeyData)?;
                Ok(Self(data))
            }
            _ => Err(IdentityError::InvalidKeyType.into()),
        }
    }
}

impl TryFrom<PublicKey> for P256ECDSAPublicKey {
    type Error = Error;

    fn try_from(value: PublicKey) -> Result<Self> {
        match value.stype() {
            SecretType::NistP256 => {
                let data = value
                    .data()
                    .try_into()
                    .map_err(|_| IdentityError::InvalidKeyData)?;
                Ok(Self(data))
            }
            _ => Err(IdentityError::InvalidKeyType.into()),
        }
    }
}
