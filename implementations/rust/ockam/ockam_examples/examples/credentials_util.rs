use ockam::{
    CredentialAttributeSchema, CredentialAttributeType, CredentialFragment2, CredentialOffer,
    CredentialPresentation, CredentialRequest, CredentialSchema, PresentationManifest, SECRET_ID,
};
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

big_array! {
    FixedArray;
    48, 96,
}

/// Messages that involve credential issuance and proving
#[derive(Debug, Deserialize, Serialize)]
pub enum CredentialMessage {
    CredentialConnection,
    NewCredential,
    CredentialIssuer {
        #[serde(with = "FixedArray")]
        public_key: [u8; 96],
        #[serde(with = "FixedArray")]
        proof: [u8; 48],
    },
    CredentialOffer(CredentialOffer),
    CredentialRequest(CredentialRequest),
    InvalidCredentialRequest,
    CredentialResponse(CredentialFragment2),
    PresentationManifest(PresentationManifest),
    Presentation(Vec<CredentialPresentation>),
}

pub fn example_schema() -> CredentialSchema {
    CredentialSchema {
        id: "file:///truck-schema-20210227-1_0_0".to_string(),
        label: "Truck Management".to_string(),
        description: "A Demoable schema".to_string(),
        attributes: vec![
            CredentialAttributeSchema {
                label: SECRET_ID.to_string(),
                description: "A unique identifier for maintenance worker. ".to_string(),
                attribute_type: CredentialAttributeType::Blob,
                unknown: true,
            },
            CredentialAttributeSchema {
                label: "can_access".to_string(),
                description: "Can worker access the truck maintenance codes?".to_string(),
                attribute_type: CredentialAttributeType::Number,
                unknown: false,
            },
        ],
    }
}
