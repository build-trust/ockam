use crate::{
    Credential, Credentials, Identities, Identity, IdentityError, IdentityIdentifier,
    SecureChannelTrustInfo, TrustContext, TrustPolicy, XXVault,
};
use ockam_core::compat::sync::Arc;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Error, Message, Result};
use ockam_vault::{KeyId, PublicKey, Signature};
use serde::{Deserialize, Serialize};
use tracing::info;

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
    pub(super) encryption_key: KeyId,
    pub(super) decryption_key: KeyId,
}

/// The end result of a handshake with identity/credentials exchange is
/// a pair of encryption/decryption keys + the identity of the other party
#[derive(Debug, Clone)]
pub(super) struct HandshakeResults {
    pub(super) handshake_keys: HandshakeKeys,
    pub(super) their_identifier: IdentityIdentifier,
}

/// This struct implements functions common to both initiator and the responder state machines
pub(super) struct CommonStateMachine {
    pub(super) vault: Arc<dyn XXVault>,
    pub(super) identities: Arc<Identities>,
    pub(super) identifier: IdentityIdentifier,
    pub(super) credentials: Vec<Credential>,
    pub(super) trust_policy: Arc<dyn TrustPolicy>,
    pub(super) trust_context: Option<TrustContext>,
    their_identifier: Option<IdentityIdentifier>,
}

impl CommonStateMachine {
    pub(super) fn new(
        vault: Arc<dyn XXVault>,
        identities: Arc<Identities>,
        identifier: IdentityIdentifier,
        credentials: Vec<Credential>,
        trust_policy: Arc<dyn TrustPolicy>,
        trust_context: Option<TrustContext>,
    ) -> Self {
        Self {
            vault,
            identities,
            identifier,
            credentials,
            trust_policy,
            trust_context,
            their_identifier: None,
        }
    }

    /// Prepare a payload containing the identity of the current party and serialize it.
    /// That payload contains:
    ///
    ///  - the current identity
    ///  - a signature of the static key used during the handshake
    ///  - the identity credentials
    ///
    pub(super) async fn make_identity_payload(&self, static_key: &KeyId) -> Result<Vec<u8>> {
        // prepare the payload that will be sent either in message 2 or message 3
        let identity = self
            .identities
            .repository()
            .get_identity(&self.identifier)
            .await?;
        let payload = IdentityAndCredentials {
            identity: identity.export()?,
            signature: self.sign_static_key(identity, static_key).await?,
            credentials: self.credentials.clone(),
        };
        Ok(serde_bare::to_vec(&payload)?)
    }

    /// Verify the identity sent by the other party: the signature and the credentials must be valid
    /// If everything is valid, store the identity identifier which will used to make the
    /// final state machine result
    pub(super) async fn verify_identity(
        &mut self,
        peer: IdentityAndCredentials,
        peer_public_key: &PublicKey,
    ) -> Result<()> {
        let identity = self.decode_identity(peer.identity).await?;
        self.verify_signature(&identity, &peer.signature, peer_public_key)
            .await?;
        self.verify_credentials(&identity, peer.credentials).await?;
        self.their_identifier = Some(identity.identifier());
        Ok(())
    }

    /// Deserialize a payload as D from a bare encoding
    pub(super) fn deserialize<D: for<'a> Deserialize<'a>>(payload: Vec<u8>) -> Result<D> {
        serde_bare::from_slice(payload.as_slice())
            .map_err(|error| Error::new(Origin::Channel, Kind::Invalid, error))
    }

    /// Sign the static key used in the key exchange with the identity private key
    async fn sign_static_key(&self, identity: Identity, key_id: &KeyId) -> Result<Signature> {
        let public_static_key = self.vault.get_public_key(key_id).await?;
        self.identities
            .identities_keys()
            .create_signature(&identity, public_static_key.data(), None)
            .await
    }

    /// Decode an Identity that was encoded with a BARE encoding
    async fn decode_identity(&self, encoded: Vec<u8>) -> Result<Identity> {
        self.identities
            .identities_creation()
            .decode_identity(encoded.as_slice())
            .await
    }

    /// Verify that the signature was signed with the public key associated to the other party identity
    async fn verify_signature(
        &self,
        their_identity: &Identity,
        their_signature: &Signature,
        their_public_key: &PublicKey,
    ) -> Result<()> {
        // verify the signature of the static key used during noise exchanges
        // actually matches the signature of the identity
        let signature_verified = self
            .identities
            .identities_keys()
            .verify_signature(
                their_identity,
                their_signature,
                their_public_key.data(),
                None,
            )
            .await?;

        if !signature_verified {
            Err(IdentityError::SecureChannelVerificationFailed.into())
        } else {
            Ok(())
        }
    }

    /// Verify that the credentials sent by the other party are valid using a trust context
    /// and store them
    async fn verify_credentials(
        &self,
        their_identity: &Identity,
        credentials: Vec<Credential>,
    ) -> Result<()> {
        // check our TrustPolicy
        let trust_info = SecureChannelTrustInfo::new(their_identity.identifier.clone());
        let trusted = self.trust_policy.check(&trust_info).await?;
        if !trusted {
            // TODO: Shutdown? Communicate error?
            return Err(IdentityError::SecureChannelTrustCheckFailed.into());
        }
        info!(
            "Initiator checked trust policy for SecureChannel from: {}",
            their_identity.identifier
        );

        if let Some(trust_context) = self.trust_context.clone() {
            for credential in credentials {
                let result = self
                    .identities
                    .receive_presented_credential(
                        &their_identity.identifier,
                        &[trust_context.authority()?.identity().await?],
                        credential,
                    )
                    .await;

                if let Some(_err) = result.err() {
                    // TODO: consider the possibility of keep going when a credential validation fails
                    return Err(IdentityError::SecureChannelVerificationFailed.into());
                }
            }
        } else if !credentials.is_empty() {
            // we cannot validate credentials without a trust context
            return Err(IdentityError::SecureChannelVerificationFailed.into());
        };

        // store identity for future validation
        self.identities
            .repository()
            .update_identity(their_identity)
            .await?;

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
#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub(super) struct IdentityAndCredentials {
    /// Exported identity
    pub(super) identity: Vec<u8>,
    /// The signature guarantees that the other end has access to the private key of the identity
    /// The signature refers to the static key of the noise ('x') and is made with the static
    /// key of the identity
    pub(super) signature: Signature,
    /// Credentials associated to the identity
    pub(super) credentials: Vec<Credential>,
}
