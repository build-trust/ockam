use ockam::{
    CredentialFragment2, CredentialOffer, CredentialPresentation, CredentialRequest,
    PresentationManifest,
};

use serde::{Deserialize, Serialize};

use serde_big_array::big_array;

big_array! {
    FixedArray;
    48, 96,
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
    Presentation(Vec<CredentialPresentation>),
}
