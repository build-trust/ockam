use super::super::models::{Credential, PurposeKeyAttestation};
use minicbor::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CredentialAndPurposeKey {
    #[n(1)] pub credential: Credential,
    #[n(2)] pub purpose_key_attestation: PurposeKeyAttestation,
}
