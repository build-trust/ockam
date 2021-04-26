mod attribute;
mod attribute_schema;
mod attribute_type;
mod error;
mod fragment1;
mod fragment2;
mod holder;
mod issuer;
mod offer;
mod presentation;
mod presentation_manifest;
mod request;
mod schema;
mod util;
mod verifier;

pub use attribute::*;
pub use attribute_schema::*;
pub use attribute_type::*;
pub use error::*;
pub use fragment1::*;
pub use fragment2::*;
pub use holder::*;
pub use issuer::*;
pub use offer::*;
pub use presentation::*;
pub use presentation_manifest::*;
pub use request::*;
pub use schema::*;
use util::*;
pub use verifier::*;

use serde::{Deserialize, Serialize};
use signature_bbs_plus::Signature;

/// A credential that can be presented
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    /// The signed attributes in the credential
    pub attributes: Vec<CredentialAttribute>,
    /// The cryptographic signature
    pub signature: Signature,
}

#[cfg(test)]
mod tests {
    use super::util::MockRng;
    use super::*;
    use ockam_core::lib::*;
    use rand::SeedableRng;
    const SEED: [u8; 16] = [3u8; 16];

    fn create_test_schema() -> CredentialSchema {
        let attribute = CredentialAttributeSchema {
            label: String::from("test_attr"),
            description: String::from("test attribute"),
            attribute_type: CredentialAttributeType::Utf8String,
            unknown: false,
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
                    unknown: true,
                },
                CredentialAttributeSchema {
                    label: String::from("device-name"),
                    description: String::from(""),
                    attribute_type: CredentialAttributeType::Utf8String,
                    unknown: false,
                },
                CredentialAttributeSchema {
                    label: String::from("manufacturer"),
                    description: String::from(""),
                    attribute_type: CredentialAttributeType::Utf8String,
                    unknown: false,
                },
                CredentialAttributeSchema {
                    label: String::from("issued"),
                    description: String::from("Unix timestamp of datetime issued"),
                    attribute_type: CredentialAttributeType::Number,
                    unknown: false,
                },
            ]
            .to_vec(),
        }
    }

    #[test]
    fn test_proof_of_possession() {
        let rng = MockRng::from_seed(SEED);
        let issuer = CredentialIssuer::new(rng);

        let proof = issuer.create_proof_of_possession();
        let pk = issuer.get_public_key();
        assert!(CredentialVerifier::verify_proof_of_possession(pk, proof));
    }

    #[test]
    fn test_credential_issuance() {
        let mut rng = MockRng::from_seed(SEED);
        let schema = get_test_issuance_schema();
        let issuer = CredentialIssuer::new(&mut rng);
        let holder = CredentialHolder::new(&mut rng);

        let pk = issuer.get_public_key();
        let offer = issuer.create_offer(&schema, &mut rng);
        let res = holder.accept_credential_offer(&offer, pk, &mut rng);
        assert!(res.is_ok());
        let (request, fragment1) = res.unwrap();
        let attributes = [
            (
                schema.attributes[1].label.clone(),
                CredentialAttribute::String(String::from("local-test")),
            ),
            (
                schema.attributes[2].label.clone(),
                CredentialAttribute::String(String::from("ockam.io")),
            ),
            (
                schema.attributes[3].label.clone(),
                CredentialAttribute::Numeric(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64,
                ),
            ),
        ];
        let res = issuer.sign_credential_request(&request, &schema, &attributes, offer.id);
        assert!(res.is_ok());
        let fragment2 = res.unwrap();
        let cred = holder.combine_credential_fragments(fragment1, fragment2);
        assert!(holder.is_valid_credential(&cred, pk));
    }

    #[test]
    fn test_credential_presentation() {
        let mut rng = MockRng::from_seed(SEED);
        let schema = get_test_issuance_schema();
        let issuer = CredentialIssuer::new(&mut rng);
        let holder = CredentialHolder::new(&mut rng);

        let cred = issuer
            .sign_credential(
                &schema,
                &[
                    CredentialAttribute::Blob(holder.id.to_bytes()),
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
        let pr_id = CredentialVerifier::create_proof_request_id(&mut rng);
        let res = holder.present_credentials(&[cred.clone()], &[manifest.clone()], pr_id, &mut rng);
        assert!(res.is_err());
        manifest.revealed = [1].to_vec();
        let res = holder.present_credentials(&[cred.clone()], &[manifest.clone()], pr_id, &mut rng);
        assert!(res.is_ok());
        let prez = res.unwrap();
        let res = CredentialVerifier::verify_credential_presentations(&prez, &[manifest], pr_id);
        assert!(res.is_ok());
    }
}
