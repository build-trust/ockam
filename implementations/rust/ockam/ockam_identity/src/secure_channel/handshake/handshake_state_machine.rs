use minicbor::{Decode, Encode};
use ockam_core::compat::string::ToString;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::{async_trait, Result};
use ockam_vault::{AeadSecretKeyHandle, X25519PublicKey};
use tracing::{debug, warn};

use crate::models::{
    ChangeHistory, CredentialAndPurposeKey, Identifier, PurposeKeyAttestation, PurposePublicKey,
};
use crate::{
    Identities, Identity, IdentityError, SecureChannelTrustInfo, TrustContext, TrustPolicy,
};

/// Interface for a state machine in a key exchange protocol
#[async_trait]
pub(super) trait StateMachine: Send + Sync + 'static {
    async fn on_event(&mut self, event: Event) -> Result<Action>;
    fn get_handshake_results(&self) -> Option<HandshakeResults>;
}

/// Events received by the state machine, either initializing the state machine
/// or receiving a message from the other party
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Event {
    Initialize,
    ReceivedMessage(Vec<u8>),
}

/// Outcome of processing an event: either no action or a message to send to the other party
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Action {
    NoAction,
    SendMessage(Vec<u8>),
}

/// List of possible states for the initiator or responder sides of the exchange
#[derive(Debug, Clone)]
pub(super) enum Status {
    Initial,
    WaitingForMessage1,
    WaitingForMessage2,
    WaitingForMessage3,
    Ready(HandshakeKeys),
}

/// At the end of a successful handshake a pair of encryption/decryption keys is available
#[derive(Debug, Clone)]
pub(super) struct HandshakeKeys {
    pub(super) encryption_key: AeadSecretKeyHandle,
    pub(super) decryption_key: AeadSecretKeyHandle,
}

/// The end result of a handshake with identity/credentials exchange is
/// a pair of encryption/decryption keys + the identity of the other party
#[derive(Debug, Clone)]
pub(super) struct HandshakeResults {
    pub(super) handshake_keys: HandshakeKeys,
    pub(super) their_identifier: Identifier,
}

/// This struct implements functions common to both initiator and the responder state machines
pub(super) struct CommonStateMachine {
    pub(super) identities: Arc<Identities>,
    pub(super) identifier: Identifier,
    pub(super) purpose_key_attestation: PurposeKeyAttestation,
    pub(super) credentials: Vec<CredentialAndPurposeKey>,
    pub(super) trust_policy: Arc<dyn TrustPolicy>,
    pub(super) trust_context: Option<TrustContext>,
    their_identifier: Option<Identifier>,
}

impl CommonStateMachine {
    pub(super) fn new(
        identities: Arc<Identities>,
        identifier: Identifier,
        purpose_key_attestation: PurposeKeyAttestation,
        credentials: Vec<CredentialAndPurposeKey>,
        trust_policy: Arc<dyn TrustPolicy>,
        trust_context: Option<TrustContext>,
    ) -> Self {
        Self {
            identities,
            identifier,
            purpose_key_attestation,
            credentials,
            trust_policy,
            trust_context,
            their_identifier: None,
        }
    }

    /// Prepare a payload containing the identity of the current party and serialize it.
    /// That payload contains:
    ///
    ///  - the current Identity Change History
    ///  - the current Secure Channel Purpose Key Attestation
    ///  - the Identity Credentials and corresponding Credentials Purpose Key Attestations
    ///
    pub(super) async fn make_identity_payload(&self) -> Result<Vec<u8>> {
        // prepare the payload that will be sent either in message 2 or message 3
        let change_history = self
            .identities
            .repository()
            .get_identity(&self.identifier)
            .await?;
        let payload = IdentityAndCredentials {
            change_history,
            purpose_key_attestation: self.purpose_key_attestation.clone(),
            credentials: self.credentials.clone(),
        };
        Ok(minicbor::to_vec(payload)?)
    }

    /// Verify the identity sent by the other party: the Purpose Key and the credentials must be valid
    /// If everything is valid, store the identity identifier which will used to make the
    /// final state machine result
    pub(super) async fn verify_identity(
        &mut self,
        peer: IdentityAndCredentials,
        peer_public_key: &X25519PublicKey,
    ) -> Result<()> {
        let identity = Identity::import_from_change_history(
            None,
            peer.change_history.clone(),
            self.identities.vault().verifying_vault,
        )
        .await?;

        self.identities
            .identities_creation()
            .update_identity(&identity)
            .await?;

        let purpose_key = self
            .identities
            .purpose_keys()
            .purpose_keys_verification()
            .verify_purpose_key_attestation(
                Some(identity.identifier()),
                &peer.purpose_key_attestation,
            )
            .await?;

        match &purpose_key.public_key {
            PurposePublicKey::SecureChannelStatic(public_key) => {
                if public_key.0 != peer_public_key.0 {
                    return Err(IdentityError::InvalidKeyData.into());
                }
            }
            PurposePublicKey::CredentialSigning(_) => {
                return Err(IdentityError::InvalidKeyType.into())
            }
        }

        self.verify_credentials(identity.identifier(), peer.credentials)
            .await?;
        self.their_identifier = Some(identity.identifier().clone());
        Ok(())
    }

    /// Verify that the credentials sent by the other party are valid using a trust context
    /// and store them
    async fn verify_credentials(
        &self,
        their_identifier: &Identifier,
        credentials: Vec<CredentialAndPurposeKey>,
    ) -> Result<()> {
        // check our TrustPolicy
        let trust_info = SecureChannelTrustInfo::new(their_identifier.clone());
        let trusted = self.trust_policy.check(&trust_info).await?;
        if !trusted {
            // TODO: Shutdown? Communicate error?
            return Err(IdentityError::SecureChannelTrustCheckFailed.into());
        }
        debug!(
            "Initiator checked trust policy for SecureChannel from: {}",
            their_identifier
        );

        if let Some(trust_context) = &self.trust_context {
            debug!(
                "got a trust context to check the credentials. There are {} credentials to check",
                credentials.len()
            );
            for credential in &credentials {
                let result = self
                    .identities
                    .credentials()
                    .credentials_verification()
                    .receive_presented_credential(
                        their_identifier,
                        &[trust_context.authority()?.identifier().clone()],
                        credential,
                    )
                    .await;

                if let Some(err) = result.err() {
                    warn!("a credential could not be validated {}", err.to_string());
                    // TODO: consider the possibility of keep going when a credential validation fails
                    return Err(
                        IdentityError::SecureChannelVerificationFailedIncorrectCredential.into(),
                    );
                }
            }
        } else if !credentials.is_empty() {
            warn!("no credentials have been received");
            // we cannot validate credentials without a trust context
            return Err(IdentityError::SecureChannelVerificationFailedMissingTrustContext.into());
        };

        Ok(())
    }

    /// Return the results of the full handshake
    ///  - the other party identity
    ///  - the encryption and decryption keys to use on the next messages to exchange
    pub(super) fn make_handshake_results(
        &self,
        handshake_keys: Option<HandshakeKeys>,
    ) -> Option<HandshakeResults> {
        match (self.their_identifier.clone(), handshake_keys) {
            (Some(their_identifier), Some(handshake_keys)) => Some(HandshakeResults {
                their_identifier,
                handshake_keys,
            }),
            _ => None,
        }
    }
}

/// This internal structure is used as a payload in the XX protocol
#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub(super) struct IdentityAndCredentials {
    /// Exported identity
    #[n(1)] pub(super) change_history: ChangeHistory,
    /// The Purpose Key guarantees that the other end has access to the private key of the identity
    /// The Purpose Key here is also the static key of the noise ('x') and is issued with the static
    /// key of the identity
    #[n(2)] pub(super) purpose_key_attestation: PurposeKeyAttestation,
    /// Credentials associated to the identity along with corresponding Credentials Purpose Keys
    /// to verify those Credentials
    #[n(3)] pub(super) credentials: Vec<CredentialAndPurposeKey>,
}
