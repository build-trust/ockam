use core::fmt;
use std::fmt::Formatter;

use minicbor::{CborLen, Encode};
use serde::{Serialize, Serializer};

use ockam::identity::models::{
    CredentialAndPurposeKey, CredentialData, CredentialVerifyingKey, PurposeKeyAttestation,
    PurposeKeyAttestationData, PurposePublicKey, VersionedData,
};
use ockam::identity::{Credential, Identifier, Identity};
use ockam_api::output::{human_readable_time, Output};

use ockam_vault::{
    ECDSASHA256CurveP256PublicKey, EdDSACurve25519PublicKey, VerifyingPublicKey, X25519PublicKey,
};

pub struct X25519PublicKeyDisplay(pub X25519PublicKey);

impl fmt::Display for X25519PublicKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "X25519: {}", hex::encode(self.0 .0))
    }
}

pub struct Ed25519PublicKeyDisplay(pub EdDSACurve25519PublicKey);

impl fmt::Display for Ed25519PublicKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Ed25519: {}", hex::encode(self.0 .0))
    }
}

pub struct P256PublicKeyDisplay(pub ECDSASHA256CurveP256PublicKey);

impl fmt::Display for P256PublicKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "P256: {}", hex::encode(self.0 .0))
    }
}

pub struct PurposePublicKeyDisplay(pub PurposePublicKey);

impl fmt::Display for PurposePublicKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            PurposePublicKey::SecureChannelStatic(key) => {
                writeln!(
                    f,
                    "Secure Channel Key -> {}",
                    X25519PublicKeyDisplay(key.clone())
                )?;
            }
            PurposePublicKey::CredentialSigning(key) => match key {
                CredentialVerifyingKey::EdDSACurve25519(key) => {
                    writeln!(
                        f,
                        "Credentials Key -> {}",
                        Ed25519PublicKeyDisplay(key.clone())
                    )?;
                }
                CredentialVerifyingKey::ECDSASHA256CurveP256(key) => {
                    writeln!(
                        f,
                        "Credentials Key -> {}",
                        P256PublicKeyDisplay(key.clone())
                    )?;
                }
            },
        }

        Ok(())
    }
}

pub struct CredentialDisplay(pub Credential);

impl fmt::Display for CredentialDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let versioned_data = match minicbor::decode::<VersionedData>(&self.0.data) {
            Ok(versioned_data) => versioned_data,
            Err(_) => {
                writeln!(f, "Invalid VersionedData")?;
                return Ok(());
            }
        };

        writeln!(f, "Version:                    {}", versioned_data.version)?;

        let credential_data = match CredentialData::get_data(&versioned_data) {
            Ok(credential_data) => credential_data,
            Err(_) => {
                writeln!(f, "Invalid CredentialData")?;
                return Ok(());
            }
        };

        if let Some(subject) = &credential_data.subject {
            writeln!(f, "Subject:                    {}", subject)?;
        }

        if let Some(subject_latest_change_hash) = &credential_data.subject_latest_change_hash {
            writeln!(
                f,
                "Subject Latest Change Hash: {}",
                subject_latest_change_hash
            )?;
        }

        writeln!(
            f,
            "Created:                    {}",
            human_readable_time(credential_data.created_at)
        )?;
        writeln!(
            f,
            "Expires:                    {}",
            human_readable_time(credential_data.expires_at)
        )?;

        writeln!(f, "Attributes: ")?;

        write!(
            f,
            "  Schema: {}; ",
            credential_data.subject_attributes.schema.0
        )?;

        f.debug_map()
            .entries(credential_data.subject_attributes.map.iter().map(|(k, v)| {
                (
                    std::str::from_utf8(k).unwrap_or("**binary**"),
                    std::str::from_utf8(v).unwrap_or("**binary**"),
                )
            }))
            .finish()?;

        Ok(())
    }
}

pub struct PurposeKeyDisplay(pub PurposeKeyAttestation);

impl fmt::Display for PurposeKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let versioned_data = match minicbor::decode::<VersionedData>(&self.0.data) {
            Ok(versioned_data) => versioned_data,
            Err(_) => {
                writeln!(f, "Invalid VersionedData")?;
                return Ok(());
            }
        };

        writeln!(f, "Version:                    {}", versioned_data.version)?;

        let purpose_key_attestation_data =
            match PurposeKeyAttestationData::get_data(&versioned_data) {
                Ok(purpose_key_attestation_data) => purpose_key_attestation_data,
                Err(_) => {
                    writeln!(f, "Invalid PurposeKeyAttestationData")?;
                    return Ok(());
                }
            };

        writeln!(
            f,
            "Subject:                    {}",
            purpose_key_attestation_data.subject
        )?;

        writeln!(
            f,
            "Subject Latest Change Hash: {}",
            purpose_key_attestation_data.subject_latest_change_hash
        )?;

        writeln!(
            f,
            "Created:                    {}",
            human_readable_time(purpose_key_attestation_data.created_at)
        )?;
        writeln!(
            f,
            "Expires:                    {}",
            human_readable_time(purpose_key_attestation_data.expires_at)
        )?;

        writeln!(
            f,
            "Public Key -> {}",
            PurposePublicKeyDisplay(purpose_key_attestation_data.public_key.clone())
        )?;

        Ok(())
    }
}

#[derive(Encode, CborLen)]
#[cbor(transparent)]
pub struct CredentialAndPurposeKeyDisplay(#[n(0)] pub CredentialAndPurposeKey);

impl Output for CredentialAndPurposeKeyDisplay {
    fn item(&self) -> ockam_api::Result<String> {
        Ok(format!("{}", self))
    }
}

impl fmt::Display for CredentialAndPurposeKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // TODO: Could borrow using a lifetime
        writeln!(f, "Credential:")?;
        writeln!(f, "{}", CredentialDisplay(self.0.credential.clone()))?;
        writeln!(f)?;
        writeln!(f, "Purpose key:")?;
        writeln!(
            f,
            "{}",
            PurposeKeyDisplay(self.0.purpose_key_attestation.clone())
        )?;

        Ok(())
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct IdentifierDisplay(pub Identifier);

impl fmt::Display for IdentifierDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Output for IdentifierDisplay {
    fn item(&self) -> ockam_api::Result<String> {
        Ok(self.to_string())
    }
}

pub struct IdentityDisplay(pub Identity);

impl Serialize for IdentityDisplay {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        serializer.serialize_bytes(&self.0.export().map_err(Error::custom)?)
    }
}

impl fmt::Display for IdentityDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Identifier: {}", self.0.identifier())?;
        for (i_num, change) in self.0.changes().iter().enumerate() {
            writeln!(f, "  Change[{}]:", i_num)?;
            writeln!(
                f,
                "    identifier:              {}",
                hex::encode(change.change_hash())
            )?;
            writeln!(
                f,
                "    primary_public_key:      {}",
                VerifyingPublicKeyDisplay(change.primary_public_key().clone())
            )?;
            writeln!(
                f,
                "    revoke_all_purpose_keys: {}",
                change.data().revoke_all_purpose_keys
            )?;
        }

        Ok(())
    }
}

impl Output for IdentityDisplay {
    fn item(&self) -> ockam_api::Result<String> {
        Ok(format!("{}", self))
    }
}

pub struct VerifyingPublicKeyDisplay(pub VerifyingPublicKey);

impl fmt::Display for VerifyingPublicKeyDisplay {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            VerifyingPublicKey::EdDSACurve25519(value) => {
                write!(f, "EdDSACurve25519: {}", hex::encode(value.0))
            }
            VerifyingPublicKey::ECDSASHA256CurveP256(value) => {
                write!(f, "ECDSASHA256CurveP256: {}", hex::encode(value.0))
            }
        }
    }
}

impl Serialize for VerifyingPublicKeyDisplay {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&match &self.0 {
            VerifyingPublicKey::EdDSACurve25519(value) => {
                format!("EdDSACurve25519: {}", hex::encode(value.0))
            }
            VerifyingPublicKey::ECDSASHA256CurveP256(value) => {
                format!("ECDSASHA256CurveP256: {}", hex::encode(value.0))
            }
        })
    }
}
