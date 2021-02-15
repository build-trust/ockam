//! Attribute based, privacy preserving, anonymous credentials.
//!
//! This crate provides the ability to issue and verify attribute based,
//! privacy preserving, anonymous credentials.
//!
//! The issuer of a credential signs a collection of statements that attest to
//! attributes of the subject of that credential. The subject (or a holder on
//! their behalf) can then selectively disclose these signed statements to a
//! verifier by presenting a cryptographic proof of knowledge of the issuer's
//! signature without revealing the actual signature or any of the other
//! statements that they didn't wish to disclose to this verifier.
//!
//! Applications can decide if a subject is authorized to take an action based
//! on the attributes of the subject that were proven to be signed by trusted
//! issuers. Since only limited and necessary information is revealed about
//! subjects this improves efficiency, security and privacy of applications.
//!
//! The main Ockam crate re-exports types defined in this crate.
#![no_std]
#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications,
    warnings
)]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
mod credential;
mod credential_attribute;
mod credential_attribute_schema;
mod credential_attribute_type;
#[cfg(feature = "std")]
mod credential_blinding;
#[cfg(feature = "std")]
mod credential_presentation;
#[cfg(feature = "std")]
mod credential_request;
mod credential_schema;
mod error;
#[cfg(feature = "std")]
mod holder;
#[cfg(feature = "std")]
mod issuer;
#[cfg(feature = "std")]
mod presentation_manifest;
mod serde;
#[cfg(feature = "std")]
mod verifier;

#[cfg(feature = "std")]
pub use credential::*;
pub use credential_attribute::CredentialAttribute;
pub use credential_attribute_schema::CredentialAttributeSchema;
pub use credential_attribute_type::CredentialAttributeType;
#[cfg(feature = "std")]
pub use credential_blinding::CredentialBlinding;
#[cfg(feature = "std")]
pub use credential_presentation::CredentialPresentation;
#[cfg(feature = "std")]
pub use credential_request::CredentialRequest;
pub use credential_schema::CredentialSchema;
#[cfg(feature = "std")]
pub use holder::*;
#[cfg(feature = "std")]
pub use issuer::Issuer;
#[cfg(feature = "std")]
pub use presentation_manifest::PresentationManifest;
#[cfg(feature = "std")]
pub use verifier::Verifier;

#[cfg(test)]
mod tests {
    use crate::*;
    use ockam_core::lib::*;

    fn create_test_schema() -> CredentialSchema {
        let attribute = CredentialAttributeSchema {
            label: String::from("test_attr"),
            description: String::from("test attribute"),
            attribute_type: CredentialAttributeType::Utf8String,
        };

        let attributes = [attribute].to_vec();

        CredentialSchema {
            id: String::from("test_id"),
            label: String::from("test_label"),
            description: String::from("test_desc"),
            attributes,
        }
    }

    #[test]
    fn test_schema_creation() {
        let _schema = create_test_schema();
    }

    #[test]
    fn test_schema_serialization() {
        let mut schema = create_test_schema();

        if let Ok(serialized) = serde_json::to_string(&schema) {
            assert!(serialized.contains("test_id"));
            assert!(serialized.contains("test_label"));
            assert!(serialized.contains("test_desc"));
            assert!(serialized.contains("test_attr"));
            assert!(serialized.contains("test attribute"));

            if let Ok(mut rehydrated) = serde_json::from_str::<CredentialSchema>(&serialized) {
                assert_eq!(rehydrated.id, schema.id);
                assert_eq!(rehydrated.label, schema.label);
                assert_eq!(rehydrated.description, schema.description);
                assert_eq!(rehydrated.attributes.len(), schema.attributes.len());

                if let Some(schema_attr) = schema.attributes.pop() {
                    if let Some(rehydrated_attr) = rehydrated.attributes.pop() {
                        assert_eq!(schema_attr.attribute_type, rehydrated_attr.attribute_type);
                        assert_eq!(schema_attr.label, rehydrated_attr.label);
                        assert_eq!(schema_attr.description, rehydrated_attr.description);
                    } else {
                        panic!("Missing rehydrated attribute")
                    }
                } else {
                    panic!("Missing Schema attribute")
                }
            }
        } else {
            panic!("Couldn't serialize Schema")
        }
    }

    #[cfg(feature = "std")]
    fn get_test_issuance_schema() -> CredentialSchema {
        CredentialSchema {
            id: String::from("test_id"),
            label: String::from("test_label"),
            description: String::from("test_desc"),
            attributes: [
                CredentialAttributeSchema {
                    label: String::from(SECRET_ID),
                    description: String::from(""),
                    attribute_type: CredentialAttributeType::Blob,
                },
                CredentialAttributeSchema {
                    label: String::from("device-name"),
                    description: String::from(""),
                    attribute_type: CredentialAttributeType::Utf8String,
                },
                CredentialAttributeSchema {
                    label: String::from("manufacturer"),
                    description: String::from(""),
                    attribute_type: CredentialAttributeType::Utf8String,
                },
                CredentialAttributeSchema {
                    label: String::from("issued"),
                    description: String::from("Unix timestamp of datetime issued"),
                    attribute_type: CredentialAttributeType::Number,
                },
            ]
            .to_vec(),
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_proof_of_possession() {
        let issuer = Issuer::new();

        let proof = issuer.create_proof_of_possession();
        let pk = issuer.get_public_key();
        assert!(Verifier::verify_proof_of_possession(pk, proof));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_credential_issuance() {
        let schema = get_test_issuance_schema();
        let issuer = Issuer::new();
        let holder = Holder::new();

        let pk = issuer.get_public_key();
        let offer = issuer.create_offer(&schema);
        let res = holder.accept_credential_offer(&offer, pk);
        assert!(res.is_ok());
        let (request, blinding) = res.unwrap();
        let mut attributes = BTreeMap::new();
        attributes.insert(
            schema.attributes[1].label.clone(),
            CredentialAttribute::String(String::from("local-test")),
        );
        attributes.insert(
            schema.attributes[2].label.clone(),
            CredentialAttribute::String(String::from("ockam.io")),
        );
        attributes.insert(
            schema.attributes[3].label.clone(),
            CredentialAttribute::Numeric(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
            ),
        );
        let res = issuer.blind_sign_credential(&request, &schema, &attributes, offer.id);
        assert!(res.is_ok());
        let bc = res.unwrap();
        let cred = holder.unblind_credential(bc, blinding);
        assert!(holder.is_valid_credential(&cred, pk));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_credential_presentation() {
        let schema = get_test_issuance_schema();
        let issuer = Issuer::new();
        let holder = Holder::new();

        let cred = issuer
            .sign_credential(
                &schema,
                &[
                    CredentialAttribute::Blob(holder.id.to_bytes_compressed_form()),
                    CredentialAttribute::String(String::from("local-test")),
                    CredentialAttribute::String(String::from("ockam.io")),
                    CredentialAttribute::Numeric(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64,
                    ),
                ],
            )
            .unwrap();

        let mut manifest = PresentationManifest {
            credential_schema: schema,
            public_key: issuer.get_public_key(),
            revealed: [0, 1].to_vec(),
        };
        let pr_id = Verifier::create_proof_request_id();
        let res = holder.present_credentials(&[cred.clone()], &[manifest.clone()], pr_id);
        assert!(res.is_err());
        manifest.revealed = [1].to_vec();
        let res = holder.present_credentials(&[cred.clone()], &[manifest.clone()], pr_id);
        assert!(res.is_ok());
        let prez = res.unwrap();
        let res = Verifier::verify_credential_presentations(prez.as_slice(), &[manifest], pr_id);
        assert!(res.is_ok());
    }
}
