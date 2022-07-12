use crate::{CowBytes, CowStr};
use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub enum CredentialRequest<'a> {
    #[n(0)] Identity {
        #[b(0)] ident: CowBytes<'a>
    }
}

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub enum Credential<'a> {
    #[n(0)] Identity {
        #[b(0)] ident: CowBytes<'a>,
        #[b(1)] signature: Signature<'a>
    }
}

impl<'a> Credential<'a> {
    pub fn to_owned<'b>(&self) -> Credential<'b> {
        match self {
            Credential::Identity { ident, signature } => Credential::Identity {
                ident: ident.to_owned(),
                signature: signature.to_owned(),
            },
        }
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
