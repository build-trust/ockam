use minicbor::{CborLen, Decode, Encode};
use tracing::{debug, warn};

use ockam_core::compat::boxed::Box;
use ockam_core::compat::string::ToString;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{async_trait, Result};
use ockam_vault::{AeadSecretKeyHandle, X25519PublicKey};

use crate::models::{
    ChangeHistory, CredentialAndPurposeKey, PurposeKeyAttestation, PurposePublicKey,
};
use crate::{
    CredentialRetriever, Identifier, Identities, IdentityError, SecureChannelTrustInfo, TrustPolicy,
};

/// Interface for a state machine in a key exchange protocol
#[async_trait]
pub(crate) trait StateMachine: Send + Sync + 'static {
    async fn on_event(&mut self, event: Event) -> Result<Action>;
    fn get_handshake_results(&self) -> Option<HandshakeResults>;
}

/// Events received by the state machine, either initializing the state machine
/// or receiving a message from the other party
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    Initialize,
    ReceivedMessage(Vec<u8>),
}

/// Outcome of processing an event: either no action or a message to send to the other party
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    NoAction,
    SendMessage(Vec<u8>),
}

/// List of possible states for the initiator or responder sides of the exchange
#[derive(Debug, Clone)]
pub(crate) enum Status {
    Initial,
    WaitingForMessage1,
    WaitingForMessage2,
    WaitingForMessage3,
    Ready(HandshakeKeys),
}

/// At the end of a successful handshake a pair of encryption/decryption keys is available
#[derive(Debug, Clone)]
pub(crate) struct HandshakeKeys {
    pub(super) encryption_key: AeadSecretKeyHandle,
    pub(super) decryption_key: AeadSecretKeyHandle,
}

/// The end result of a handshake with identity/credentials exchange is
/// a pair of encryption/decryption keys + the identity of the other party
#[derive(Debug, Clone)]
pub(crate) struct HandshakeResults {
    pub(super) handshake_keys: HandshakeKeys,
    pub(super) their_identifier: Identifier,
    pub(super) presented_credential: Option<CredentialAndPurposeKey>,
}

/// This struct implements functions common to both initiator and the responder state machines
pub(crate) struct CommonStateMachine {
    pub(super) identities: Arc<Identities>,
    pub(super) identifier: Identifier,
    pub(super) purpose_key_attestation: PurposeKeyAttestation,
    pub(super) credential_retriever: Option<Arc<dyn CredentialRetriever>>,
    pub(super) trust_policy: Arc<dyn TrustPolicy>,
    pub(super) authority: Option<Identifier>, // TODO: Replace with ABAC
    pub(super) presented_credential: Option<CredentialAndPurposeKey>,
    their_identifier: Option<Identifier>,
}

impl CommonStateMachine {
    pub(super) fn new(
        identities: Arc<Identities>,
        identifier: Identifier,
        purpose_key_attestation: PurposeKeyAttestation,
        credential_retriever: Option<Arc<dyn CredentialRetriever>>,
        trust_policy: Arc<dyn TrustPolicy>,
        authority: Option<Identifier>,
    ) -> Self {
        Self {
            identities,
            identifier,
            purpose_key_attestation,
            credential_retriever,
            trust_policy,
            authority,
            presented_credential: None,
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
    pub(super) async fn make_identity_payload(&mut self) -> Result<Vec<u8>> {
        // prepare the payload that will be sent either in message 2 or message 3
        let change_history = self.identities.get_change_history(&self.identifier).await?;
        let credential = match &self.credential_retriever {
            Some(credential_retriever) => Some(credential_retriever.retrieve().await?),
            None => None,
        };

        self.presented_credential = credential.clone();
        let credentials = credential.map(|c| vec![c]).unwrap_or(vec![]);

        let payload = IdentityAndCredentials {
            change_history,
            purpose_key_attestation: self.purpose_key_attestation.clone(),
            credentials,
        };
        ockam_core::cbor_encode_preallocate(payload)
    }

    /// Verify the identity sent by the other party: the Purpose Key and the credentials must be valid
    /// If everything is valid, store the identity identifier which will used to make the
    /// final state machine result
    pub(super) async fn process_identity_payload(
        &mut self,
        peer: IdentityAndCredentials,
        peer_public_key: X25519PublicKey,
    ) -> Result<()> {
        let identifier = Self::process_identity_payload_static(
            self.identities.clone(),
            Some(self.trust_policy.clone()),
            self.authority.clone(),
            None,
            peer.change_history,
            peer.credentials,
            Some((peer.purpose_key_attestation, peer_public_key)),
        )
        .await?;

        self.their_identifier = Some(identifier);

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
                presented_credential: self.presented_credential.clone(),
            }),
            _ => None,
        }
    }
}

impl CommonStateMachine {
    /// Verify the identity sent by the other party: the Purpose Key and the credentials must be valid
    /// If everything is valid, store the identity identifier which will used to make the
    /// final state machine result
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn process_identity_payload_static(
        identities: Arc<Identities>,
        trust_policy: Option<Arc<dyn TrustPolicy>>,
        authority: Option<Identifier>,
        expected_identifier: Option<Identifier>,
        change_history: ChangeHistory,
        credentials: Vec<CredentialAndPurposeKey>,
        // Has value if it's the identity payload during the handshake and not credential refresh
        peer_public_key: Option<(PurposeKeyAttestation, X25519PublicKey)>,
    ) -> Result<Identifier> {
        let their_identifier = identities
            .identities_verification()
            .import_from_change_history(expected_identifier.as_ref(), change_history.clone())
            .await?;

        if let Some((purpose_key_attestation, peer_public_key)) = peer_public_key {
            let purpose_key = identities
                .purpose_keys()
                .purpose_keys_verification()
                .verify_purpose_key_attestation(Some(&their_identifier), &purpose_key_attestation)
                .await?;

            match &purpose_key.public_key {
                PurposePublicKey::SecureChannelStatic(public_key) => {
                    if public_key.0 != peer_public_key.0 {
                        return Err(IdentityError::InvalidKeyData)?;
                    }
                }
                PurposePublicKey::CredentialSigning(_) => {
                    return Err(IdentityError::InvalidKeyType)?;
                }
            }
        }

        Self::check_trust_policy(trust_policy, &their_identifier).await?;
        Self::verify_credentials(identities, authority, &their_identifier, credentials).await?;

        Ok(their_identifier)
    }

    /// Verify that the credentials sent by the other party are valid using
    async fn check_trust_policy(
        trust_policy: Option<Arc<dyn TrustPolicy>>,
        their_identifier: &Identifier,
    ) -> Result<()> {
        if let Some(trust_policy) = trust_policy {
            // check our TrustPolicy
            let trust_info = SecureChannelTrustInfo::new(their_identifier.clone());
            let trusted = trust_policy.check(&trust_info).await?;
            if !trusted {
                // TODO: Shutdown? Communicate error?
                return Err(IdentityError::SecureChannelTrustCheckFailed)?;
            }
            debug!(
                "Checked trust policy for SecureChannel from: {}",
                their_identifier
            );
        }

        Ok(())
    }

    /// Verify that the credentials sent by the other party are valid
    async fn verify_credentials(
        identities: Arc<Identities>,
        // TODO: Do we really care if the authority is known here?.
        //       Having Authority's change history in the storage is enough to verify credentials
        //       Checking whether that's the right authority may be actually better at ABAC level
        //       Also, ABAC will be used here as well
        authority: Option<Identifier>,
        their_identifier: &Identifier,
        credentials: Vec<CredentialAndPurposeKey>,
    ) -> Result<()> {
        debug!("verifying {} credentials", credentials.len());

        // Let's complete the handshake and keep the secure channel open even if we could not
        // verify credentials. The individual resources' access controls won't allow
        // unauthorized users to interact with them. However, there can be other resources that
        // allow users without credential to interact.

        let Some(authority) = &authority else {
            if !credentials.is_empty() {
                warn!("credentials were presented, but Authority is missing");
            }
            return Ok(());
        };

        if credentials.is_empty() {
            debug!(
                "no credentials were received from {}. Expected authority: {}",
                their_identifier, authority
            );
            return Ok(());
        };

        for credential in &credentials {
            let res = identities
                .credentials()
                .credentials_verification()
                .receive_presented_credential(their_identifier, &[authority.clone()], credential)
                .await;

            match res {
                Ok(_) => {
                    debug!(
                        "Successfully validated credential from {}",
                        their_identifier,
                    );
                }
                Err(err) => {
                    warn!(
                        "a credential from {} could not be validated {}",
                        their_identifier,
                        err.to_string()
                    );
                }
            }
        }

        Ok(())
    }
}

/// This internal structure is used as a payload in the XX protocol
#[derive(Debug, Clone, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub(super) struct IdentityAndCredentials {
    /// Exported identity
    #[n(0)] pub(super) change_history: ChangeHistory,
    /// The Purpose Key guarantees that the other end has access to the private key of the identity
    /// The Purpose Key here is also the static key of the noise ('x') and is issued with the static
    /// key of the identity
    #[n(1)] pub(super) purpose_key_attestation: PurposeKeyAttestation,
    /// Credentials associated to the identity along with corresponding Credentials Purpose Keys
    /// to verify those Credentials
    #[n(2)] pub(super) credentials: Vec<CredentialAndPurposeKey>,
}
