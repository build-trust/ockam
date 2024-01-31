use core::ops::Deref;

use ockam::identity::{Identifier, TimestampInSeconds};
use ockam_core::compat::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct PreTrustedIdentity {
    attrs: BTreeMap<Vec<u8>, Vec<u8>>,
    added_at: TimestampInSeconds,
    expires_at: Option<TimestampInSeconds>,
    attested_by: Identifier,
}

impl PreTrustedIdentity {
    pub fn attrs(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        &self.attrs
    }
    pub fn added_at(&self) -> TimestampInSeconds {
        self.added_at
    }
    pub fn expires_at(&self) -> Option<TimestampInSeconds> {
        self.expires_at
    }
    pub fn attested_by(&self) -> &Identifier {
        &self.attested_by
    }
    pub fn new(
        attrs: BTreeMap<Vec<u8>, Vec<u8>>,
        added_at: TimestampInSeconds,
        expires_at: Option<TimestampInSeconds>,
        attested_by: Identifier,
    ) -> Self {
        Self {
            attrs,
            added_at,
            expires_at,
            attested_by,
        }
    }
}

#[derive(Default, Clone, Debug)]
pub struct PreTrustedIdentities(BTreeMap<Identifier, PreTrustedIdentity>);

impl Deref for PreTrustedIdentities {
    type Target = BTreeMap<Identifier, PreTrustedIdentity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PreTrustedIdentities {
    pub fn new(h: BTreeMap<Identifier, PreTrustedIdentity>) -> Self {
        Self(h)
    }
}

impl From<BTreeMap<Identifier, PreTrustedIdentity>> for PreTrustedIdentities {
    fn from(h: BTreeMap<Identifier, PreTrustedIdentity>) -> PreTrustedIdentities {
        Self::new(h)
    }
}
