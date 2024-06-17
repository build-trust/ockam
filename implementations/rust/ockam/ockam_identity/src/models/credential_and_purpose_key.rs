use minicbor::{CborLen, Decode, Encode};

use ockam_core::compat::string::String;
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};

use crate::alloc::string::ToString;
use crate::models::{Credential, CredentialData, PurposeKeyAttestation};
use crate::TimestampInSeconds;

/// [`Credential`] and the corresponding [`PurposeKeyAttestation`] that was used to issue that
/// [`Credential`] and will be used to verify it
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct CredentialAndPurposeKey {
    /// [`Credential`]
    #[n(0)] pub credential: Credential,
    /// Corresponding [`PurposeKeyAttestation`] that was used to issue that
    /// [`Credential`] and will be used to verify it
    #[n(1)] pub purpose_key_attestation: PurposeKeyAttestation,
}

impl CredentialAndPurposeKey {
    /// Encode the credential as a hex String
    pub fn encode_as_string(&self) -> Result<String> {
        Ok(hex::encode(self.encode_as_cbor_bytes()?))
    }

    /// Encode the credential as a CBOR bytes
    pub fn encode_as_cbor_bytes(&self) -> Result<Vec<u8>> {
        ockam_core::cbor_encode_preallocate(self)
    }

    /// Decode the credential from bytes
    pub fn decode_from_cbor_bytes(bytes: &[u8]) -> Result<CredentialAndPurposeKey> {
        Ok(minicbor::decode(bytes)?)
    }

    /// Decode the credential from an hex string
    pub fn decode_from_string(as_hex: &str) -> Result<CredentialAndPurposeKey> {
        let hex_decoded = hex::decode(as_hex.as_bytes())
            .map_err(|e| Error::new(Origin::Api, Kind::Serialization, e.to_string()))?;
        Self::decode_from_cbor_bytes(&hex_decoded)
    }

    /// Return the encoded credential data
    pub fn get_credential_data(&self) -> Result<CredentialData> {
        self.credential.get_credential_data()
    }

    /// Return the `expires_at` field
    pub fn get_expires_at(&self) -> Result<TimestampInSeconds> {
        Ok(self.get_credential_data()?.expires_at)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::identities;
    use crate::models::CredentialSchemaIdentifier;
    use crate::utils::AttributesBuilder;

    use super::*;

    #[tokio::test]
    async fn test_encode_decode_as_bytes() -> Result<()> {
        let credential = create_credential().await?;
        let decoded = CredentialAndPurposeKey::decode_from_cbor_bytes(
            &credential.encode_as_cbor_bytes().unwrap(),
        );
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap(), credential);

        Ok(())
    }

    #[tokio::test]
    async fn test_encode_decode_as_string() -> Result<()> {
        let credential = create_credential().await?;
        let decoded =
            CredentialAndPurposeKey::decode_from_string(&credential.encode_as_string().unwrap());
        assert!(decoded.is_ok());
        assert_eq!(decoded.unwrap(), credential);

        Ok(())
    }

    /// HELPERS
    async fn create_credential() -> Result<CredentialAndPurposeKey> {
        let identities = identities().await?;
        let issuer = identities.identities_creation().create_identity().await?;
        let subject = identities.identities_creation().create_identity().await?;

        let attributes = AttributesBuilder::with_schema(CredentialSchemaIdentifier(1))
            .with_attribute("name".as_bytes().to_vec(), b"value".to_vec())
            .build();

        identities
            .credentials()
            .credentials_creation()
            .issue_credential(&issuer, &subject, attributes, Duration::from_secs(1))
            .await
    }
}
