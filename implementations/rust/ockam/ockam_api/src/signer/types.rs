use crate::{CowBytes, CowStr};
use core::fmt;
use data_encoding::BASE32_DNSSEC;
use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

#[cfg(feature = "tag")]
use crate::TypeTag;

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Credential<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3796735>,
    #[b(1)] attributes: CowBytes<'a>,
    #[b(2)] signature: Signature<'a>
}

impl<'a> Credential<'a> {
    pub(super) fn new(attributes: CowBytes<'a>, signature: Signature<'a>) -> Self {
        Credential {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attributes,
            signature,
        }
    }

    pub fn attributes(&self) -> &[u8] {
        &self.attributes
    }

    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    pub fn to_owned<'b>(&self) -> Credential<'b> {
        Credential {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attributes: self.attributes.to_owned(),
            signature: self.signature.to_owned(),
        }
    }
}

impl fmt::Display for Credential<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The derived `Encode` impl does not error and writing to a vector does not either:
        let bytes = minicbor::to_vec(self).expect("encoding a credential never fails");
        f.write_str(&BASE32_DNSSEC.encode(&bytes))
    }
}

impl TryFrom<&str> for Credential<'_> {
    type Error = (); // TODO

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let b = BASE32_DNSSEC.decode(value.as_bytes()).map_err(|_| ())?;
        let c = minicbor::decode::<Credential>(&b).map_err(|_| ())?;
        Ok(c.to_owned())
    }
}

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
pub struct Signature<'a> {
    #[b(0)] ident: IdentityId<'a>,
    #[b(1)] signature: CowBytes<'a>
}

impl<'a> Signature<'a> {
    pub fn new<S>(id: IdentityId<'a>, sig: S) -> Self
    where
        S: Into<Cow<'a, [u8]>>,
    {
        Signature {
            ident: id,
            signature: CowBytes(sig.into()),
        }
    }

    pub fn identity(&self) -> &IdentityId {
        &self.ident
    }

    pub fn data(&self) -> &[u8] {
        &self.signature
    }

    pub fn to_owned<'b>(&self) -> Signature<'b> {
        Signature {
            ident: self.ident.to_owned(),
            signature: self.signature.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Encode, Decode)]
#[cbor(transparent)]
pub struct IdentityId<'a>(#[b(0)] pub CowStr<'a>);

impl<'a> IdentityId<'a> {
    pub fn new<S: Into<Cow<'a, str>>>(id: S) -> Self {
        IdentityId(CowStr(id.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_owned<'b>(&self) -> IdentityId<'b> {
        IdentityId(self.0.to_owned())
    }
}
