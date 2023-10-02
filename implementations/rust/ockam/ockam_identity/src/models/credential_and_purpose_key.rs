use crate::models::{Credential, PurposeKeyAttestation};
use minicbor::{Decode, Encode};

/// [`Credential`] and the corresponding [`PurposeKeyAttestation`] that was used to issue that
/// [`Credential`] and will be used to verify it
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
pub struct CredentialAndPurposeKey {
    /// [`Credential`]
    #[n(0)] pub credential: Credential,
    /// Corresponding [`PurposeKeyAttestation`] that was used to issue that
    /// [`Credential`] and will be used to verify it
    #[n(1)] pub purpose_key_attestation: PurposeKeyAttestation,
}
