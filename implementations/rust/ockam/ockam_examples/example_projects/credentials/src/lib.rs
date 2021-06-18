use ockam::{
    CredentialAttributeSchema, CredentialAttributeType, CredentialPresentation, CredentialSchema,
};
use serde::{Deserialize, Serialize};
use std::io::stdin;

#[derive(Serialize, Deserialize)]
pub struct CredentialRequestOffer;

#[derive(Serialize, Deserialize)]
pub struct CredentialOffer(pub ockam::CredentialOffer);

#[derive(Serialize, Deserialize)]
pub struct CredentialRequest(pub ockam::CredentialRequest);

#[derive(Serialize, Deserialize)]
pub struct CredentialResponse(pub ockam::CredentialFragment2);

#[derive(Serialize, Deserialize)]
pub struct DoorOpenRequest;

#[derive(Serialize, Deserialize)]
pub struct DoorOpenRequestId(pub [u8; 32]);

#[derive(Serialize, Deserialize)]
pub struct DoorCredentialPresentation(pub Vec<CredentialPresentation>);

pub fn door_schema() -> CredentialSchema {
    CredentialSchema {
        id: "Office".to_string(),
        label: String::new(),
        description: String::new(),
        attributes: vec![
            CredentialAttributeSchema {
                label: "door_id".to_string(),
                description: String::new(),
                unknown: false,
                attribute_type: CredentialAttributeType::Utf8String,
            },
            CredentialAttributeSchema {
                label: "can_open_door".to_string(),
                description: "Is allowed to open the door identified by door_device_id".to_string(),
                unknown: false,
                attribute_type: CredentialAttributeType::Number,
            },
            CredentialAttributeSchema {
                label: "secret_id".to_string(),
                description: "secret id".to_string(),
                unknown: true,
                attribute_type: CredentialAttributeType::Number,
            },
        ],
    }
}

pub fn read_line() -> String {
    let mut line = String::new();
    stdin().read_line(&mut line).unwrap();
    line.replace(&['\n', '\r'][..], "")
}
