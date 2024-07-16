//! Credential request/response types

use minicbor::{CborLen, Decode, Encode};
use ockam_multiaddr::MultiAddr;

#[derive(Clone, Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct GetCredentialRequest {
    #[n(1)] overwrite: bool,
    #[n(2)] pub identity_name: Option<String>,
}

impl GetCredentialRequest {
    pub fn new(overwrite: bool, identity_name: Option<String>) -> Self {
        Self {
            overwrite,
            identity_name,
        }
    }

    pub fn is_overwrite(&self) -> bool {
        self.overwrite
    }
}

#[derive(Clone, Debug, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PresentCredentialRequest {
    #[b(1)] pub route: String,
    #[n(2)] pub oneway: bool,
}

impl PresentCredentialRequest {
    pub fn new(route: &MultiAddr, oneway: bool) -> Self {
        Self {
            route: route.to_string(),
            oneway,
        }
    }
}
