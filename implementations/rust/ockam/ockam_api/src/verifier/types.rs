use minicbor::{Decode, Encode};
use ockam_core::compat::borrow::Cow;
use ockam_core::CowBytes;
use ockam_identity::credential::{Attributes, Timestamp};
use ockam_identity::IdentityIdentifier;
use std::collections::BTreeMap;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct VerifyRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4592146>,
    #[b(1)] cred: CowBytes<'a>,
    #[n(2)] subj: IdentityIdentifier,
    #[b(3)] auth: BTreeMap<IdentityIdentifier, CowBytes<'a>>
}

#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct VerifyResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6845123>,
    #[b(1)] attrs: Attributes<'a>,
    #[n(2)] expires: Timestamp
}

impl<'a> VerifyRequest<'a> {
    pub fn new<C: Into<Cow<'a, [u8]>>>(cred: C, subj: IdentityIdentifier) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            cred: CowBytes(cred.into()),
            subj,
            auth: BTreeMap::new(),
        }
    }

    pub fn with_authority<T>(mut self, id: IdentityIdentifier, identity: T) -> Self
    where
        T: Into<Cow<'a, [u8]>>,
    {
        self.auth.insert(id, CowBytes(identity.into()));
        self
    }

    pub fn credential(&self) -> &[u8] {
        &self.cred
    }

    pub fn subject(&self) -> &IdentityIdentifier {
        &self.subj
    }

    pub fn authorities(&self) -> &BTreeMap<IdentityIdentifier, CowBytes<'a>> {
        &self.auth
    }

    pub fn authority(&self, id: &IdentityIdentifier) -> Option<&CowBytes<'a>> {
        self.auth.get(id)
    }
}

impl<'a> VerifyResponse<'a> {
    pub fn new(attrs: Attributes<'a>, expires: Timestamp) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attrs,
            expires,
        }
    }

    pub fn attributes(&self) -> &Attributes<'a> {
        &self.attrs
    }

    pub fn expires_at(&self) -> Timestamp {
        self.expires
    }
}
