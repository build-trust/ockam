use ockam::{
    CredentialFragment2, CredentialOffer, CredentialPresentation, CredentialRequest,
    PresentationManifest,
};
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;
use std::net::SocketAddr;

big_array! {
    FixedArray;
    48, 96,
}

pub type RequestId = [u8; 32];
pub static DEFAULT_ISSUER_PORT: usize = 7967;
pub static DEFAULT_VERIFIER_PORT: usize = DEFAULT_ISSUER_PORT + 1;

pub fn on<S: ToString>(host: S, port: usize) -> SocketAddr {
    format!("{}:{}", host.to_string(), port).parse().unwrap()
}

pub fn default_issuer_address() -> SocketAddr {
    on("127.0.0.1", DEFAULT_ISSUER_PORT)
}

pub fn issuer_on_or_default<S: ToString>(host: Option<S>) -> SocketAddr {
    if let Some(host) = host {
        let host = host.to_string();
        if let Some(_) = host.find(":") {
            host.parse().unwrap()
        } else {
            on(host, DEFAULT_ISSUER_PORT)
        }
    } else {
        default_issuer_address()
    }
}

use ockam::{CredentialAttributeSchema, CredentialAttributeType, CredentialSchema, SECRET_ID};

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
    Presentation(Vec<CredentialPresentation>, RequestId),
}
