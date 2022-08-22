use crate::credential::{Credential, CredentialBuilder, CredentialData, Timestamp, Verified};
use crate::{Identity, IdentityStateConst, IdentityVault};
use core::marker::PhantomData;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::SignatureVec;
use ockam_core::{CowStr, Error, Result};

impl<V: IdentityVault> Identity<V> {
    /// Create a signed credential based on the given values.
    #[cfg(feature = "std")]
    pub async fn issue_credential<'a>(
        &self,
        builder: CredentialBuilder<'a>,
    ) -> Result<Credential<'a>> {
        let key_label = IdentityStateConst::ROOT_LABEL;
        let now = Timestamp::now()
            .ok_or_else(|| Error::new(Origin::Core, Kind::Internal, "invalid system time"))?;
        let exp = Timestamp(u64::from(now).saturating_add(builder.validity.as_secs()));
        let dat = CredentialData {
            schema: builder.schema,
            attributes: builder.attrs,
            subject: builder.subject,
            issuer: self.identifier().clone(),
            issuer_key_label: CowStr(key_label.into()),
            created: now,
            expires: exp,
            status: None::<PhantomData<Verified>>,
        };
        let bytes = minicbor::to_vec(&dat)?;

        let sig = self.create_signature(&bytes, None).await?;
        Ok(Credential::new(bytes, SignatureVec::from(sig)))
    }
}
