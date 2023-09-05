use crate::models::{Credential, PurposeKeyAttestation};
use minicbor::{Decode, Encode};

/// [`Credential`] and the corresponding [`PurposeKeyAttestation`] that was used to issue that
/// [`Credential`] and will be used to verify it
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CredentialAndPurposeKey {
    /// [`Credential`]
    #[n(1)] pub credential: Credential,
    /// Corresponding [`PurposeKeyAttestation`] that was used to issue that
    /// [`Credential`] and will be used to verify it
    #[n(2)] pub purpose_key_attestation: PurposeKeyAttestation,
}
