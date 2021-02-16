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

#[cfg(test)]
mod tests {
    use crate::{CredentialAttributeSchema, CredentialAttributeType, CredentialSchema};
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
}
