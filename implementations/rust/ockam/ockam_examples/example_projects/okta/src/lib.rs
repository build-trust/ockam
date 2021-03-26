#[macro_use]
extern crate serde_big_array;

big_array! { BigArray; }

use ockam::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Messages {
    OktaLogin(usize),
    OktaLoginUrl { preamble: String, url: String },
    OktaGrantToken { token: String },
    OktaAccessToken { token: String },
    OktaRequest { token: String, msg: OckamMessages },
    OktaResponse { msg: OckamMessages },
    NonOktaRequest(OckamMessages),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OckamMessages {
    AccessDenied,
    BecomeRequest {
        role: OckamRole,
    },
    BecomeResponse {
        result: bool,
        msg: String,
    },
    ListServicesRequest {
        limit: u32,
        offset: u32,
    },
    ListServicesResponse {
        services: Vec<OckamService>,
    },
    GetEstablishmentBundlesRequest {
        services: Vec<u32>,
    },
    GetEstablishmentBundlesResponse {
        services: Vec<EstablishmentBundle>,
    },
    BeginDeviceEnrollment {
        nonce: [u8; 16],
    },
    DeviceEnrollmentRequest {
        nonce: [u8; 16],
        blind_device_secret: [u8; 32],
        proof_of_secret: [u8; 32],
    },
    DeviceEnrollmentResponse {
        schema: CredentialSchema,
        service: EstablishmentBundle,
        attributes: Vec<Vec<u8>>,
        attestation: Vec<u8>,
    },
    ServiceEnrollmentMessage1(Vec<u8>),
    ServiceEnrollmentMessage2(Vec<u8>),
    ServiceEnrollmentMessage3(Vec<u8>),
    ServiceEnrollmentResponse(Vec<u8>),
    GeneralMessage(Vec<u8>),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum OckamRole {
    Enroller {
        public_key: [u8; 32],
        #[serde(with = "BigArray")]
        proof: [u8; 64],
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum KeyEstablishment {
    Xx,
    X3dh,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OckamService {
    pub id: u32,
    pub key_establishment: Vec<KeyEstablishment>,
    pub schemas: Vec<CredentialSchema>,
}

//#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
//pub struct CredentialSchema {
//    pub id: String,
//    pub attributes: Vec<String>,
//}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct EstablishmentBundle {
    pub service_id: u32,
    pub address: String,
    pub key_establishment: KeyEstablishment,
    pub key_establishment_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Attestation {
    pub attributes: Vec<Vec<u8>>,
    pub signature: Vec<u8>,
}
