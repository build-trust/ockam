use core::marker::PhantomData;
use core::time::Duration;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::string::String;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};

use crate::credential::{
    Attributes, CredentialData, SchemaId, Timestamp, Verified, MAX_CREDENTIAL_VALIDITY,
};
use crate::identity::identity_change::IdentityChangeConstants;
use crate::identity::IdentityIdentifier;

#[cfg(feature = "tag")]
use crate::TypeTag;

/// Convenience structure to create [`Credential`]s.
pub struct CredentialBuilder {
    pub(crate) schema: Option<SchemaId>,
    pub(crate) attrs: Attributes,
    pub(crate) subject: IdentityIdentifier,
    pub(crate) issuer: IdentityIdentifier,
    pub(crate) validity: Duration,
}

impl CredentialBuilder {
    pub(super) fn new(
        subject: IdentityIdentifier,
        issuer: IdentityIdentifier,
    ) -> CredentialBuilder {
        Self {
            schema: None,
            attrs: Attributes::default(),
            subject,
            issuer,
            validity: MAX_CREDENTIAL_VALIDITY,
        }
    }

    ///
    pub fn from_attributes(
        subject: IdentityIdentifier,
        issuer: IdentityIdentifier,
        attrs: HashMap<String, String>,
    ) -> Self {
        attrs
            .iter()
            .fold(CredentialData::builder(subject, issuer), |crd, (k, v)| {
                crd.with_attribute(k, v.as_bytes())
            })
    }

    /// Add some key-value pair as credential attribute.
    pub fn with_attribute(mut self, k: &str, v: &[u8]) -> Self {
        self.attrs.put(k, v);
        self
    }

    /// Set the schema identifier of the credential.
    pub fn with_schema(mut self, s: SchemaId) -> Self {
        self.schema = Some(s);
        self
    }

    /// Set validity duration of the credential.
    ///
    /// # Panics
    ///
    /// If the given validity exceeds [`MAX_CREDENTIAL_VALIDITY`].
    pub fn valid_for(mut self, val: Duration) -> Self {
        assert! {
            val <= MAX_CREDENTIAL_VALIDITY,
            "validity exceeds allowed maximum"
        }
        self.validity = val;
        self
    }

    /// Return a verified credential data, with a created timestamp
    pub fn build(self) -> Result<CredentialData<Verified>> {
        let key_label = IdentityChangeConstants::ROOT_LABEL;
        let now = Timestamp::now()
            .ok_or_else(|| Error::new(Origin::Core, Kind::Internal, "invalid system time"))?;
        let exp = Timestamp::add_seconds(&now, self.validity.as_secs());

        Ok(CredentialData {
            schema: self.schema,
            attributes: self.attrs,
            subject: self.subject,
            issuer: self.issuer,
            issuer_key_label: key_label.into(),
            created: now,
            expires: exp,
            status: None::<PhantomData<Verified>>,
        })
    }
}
