use crate::secure_channel::handshake::handshake_state::Handshake;
use crate::secure_channel::handshake::handshake_state_machine::Action::SendMessage;
use crate::secure_channel::handshake::handshake_state_machine::Event::ReceivedMessage;
use crate::secure_channel::handshake::handshake_state_machine::{
    Action, EncodedPublicIdentity, Event, IdentityAndCredential, StateMachine,
};
use crate::{Credential, Identities, Identity, TrustContext, TrustPolicy};
use delegate::delegate;
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::{KeyId, PublicKey, SecretType, Signature, CURVE25519_PUBLIC_LENGTH_USIZE};
use ockam_core::{CompletedKeyExchange, Error, Result};
use ockam_key_exchange_xx::{XXVault, AES_GCM_TAGSIZE_USIZE};
use Event::*;
use InitiatorStatus::*;

#[async_trait]
impl StateMachine for InitiatorStateMachine {
    async fn on_event(&mut self, event: Event) -> Result<Action> {
        let mut state = self.handshake.state.clone();
        match (state.status, event) {
            (Initial, Initialize) => {
                // 1. Generate a static key pair for this handshake and set it to `s`
                state.s = self.generate_static_key().await?;
                // 2. Generate an ephemeral key pair for this handshake and set it to e
                state.e = self.generate_ephemeral_key().await?;
                // 3. Set k to empty, Set n to 0
                state.k = "".into();
                state.n = 0;

                // 4. Set h and ck to protocol name
                // prologue is empty
                state.h = self.get_protocol_name().clone();
                state.ck = self
                    .create_ephemeral_secret(self.get_protocol_name().to_vec())
                    .await?;
                state.prologue = vec![];

                // 5. h = SHA256(concat(h, prologue))
                state.h = self.mix_hash(&state.h, state.prologue.as_slice());

                // 6. h = SHA256(concat(h, e.pubKey))
                //    writeToMessage(bigendian(e.pubKey)
                let ephemeral_public_key = self.get_ephemeral_public_key(&state.e).await?;
                let output = ephemeral_public_key.data().to_vec();
                state.h = self.mix_hash(&state.h, ephemeral_public_key.data());

                // 7. payload = empty string
                //    h = SHA256(concat(h, payload))
                let payload = &[];
                state.h = self.mix_hash(&state.h, payload.as_ref());

                // Set the new status and return the action
                state.status = WaitingForMessage2;
                self.handshake.state = state;
                Ok(SendMessage(output))
            }
            // Process the message sent by the responder and send the last message
            (WaitingForMessage2, ReceivedMessage(message)) => {
                // -------
                // RECEIVE
                // -------
                // 1. re = readFromMessage(32 bytes)
                //    h = SHA256(concat(h, re))
                let re = PublicKey::new(
                    self.read_from_message(&message, 0, CURVE25519_PUBLIC_LENGTH_USIZE)?
                        .to_vec(),
                    SecretType::X25519,
                );

                state.h = self.mix_hash(&state.h, re.data());

                // 2. ck, k = HKDF(ck, DH(e, re), 2)
                // n = 0
                let dh = self.generate_diffie_hellman_key(&state.e, &re).await?;
                let (ck, k) = self.hkdf(&state.ck, &dh).await?;
                state.ck = self.replace_key(&state.ck, &ck).await?;
                state.k = self.replace_key(&state.k, &k).await?;
                state.n = 0;

                // 3. c = readFromMessage(48 bytes)
                //    h = SHA256(concat(h, c))
                let c = self.read_from_message(
                    &message,
                    CURVE25519_PUBLIC_LENGTH_USIZE,
                    CURVE25519_PUBLIC_LENGTH_USIZE + AES_GCM_TAGSIZE_USIZE,
                )?;
                state.h = self.mix_hash(&state.h, c);

                // 4. rs = DECRYPT(k, n++, h, c)
                state.n += 1;
                let rs = PublicKey::new(
                    self.decrypt(&state.k, state.n, &state.h, c).await?,
                    SecretType::X25519,
                );

                // 5. ck, k = HKDF(ck, DH(e, rs), 2)
                let dh = self.generate_diffie_hellman_key(&state.e, &rs).await?;
                let (ck, k) = self.hkdf(&state.ck, &dh).await?;
                state.ck = self.replace_key(&state.ck, &ck).await?;
                state.k = self.replace_key(&state.k, &k).await?;

                // 6. c = readRemainingMessage()
                //    h = SHA256(concat(h, c))
                let c = self.read_end_of_message(
                    &message,
                    CURVE25519_PUBLIC_LENGTH_USIZE * 2 + AES_GCM_TAGSIZE_USIZE,
                )?;
                state.h = self.mix_hash(&state.h, c);

                // 7. payload = DECRYPT(k, n++, h, c)
                state.n += 1;

                let payload = self.decrypt(&state.k, state.n, &state.h, c).await?;
                let identity_and_credential: IdentityAndCredential =
                    serde_bare::from_slice(payload.as_slice())
                        .map_err(|error| Error::new(Origin::Channel, Kind::Invalid, error))?;

                // ----
                // SEND
                // ----

                // 1. ENCRYPT(k, n++, h, s.pubKey)
                //    h = SHA256(concat(h, c))
                state.n += 1;
                let s_public_key = self.get_ephemeral_public_key(&state.s).await?;
                let c = self
                    .encrypt(&state.k, state.n, &state.h, &s_public_key.data())
                    .await?;
                state.h = self.mix_hash(&state.h, c.as_slice());

                // 2. writeToMessage(bigendian(c))
                let message_to_send = c.to_vec();

                // 3. ck, k = HKDF(ck, DH(s, re), 2)
                //    h = SHA256(concat(h, c))
                let dh = self.generate_diffie_hellman_key(&state.s, &re).await?;
                let (ck, k) = self.hkdf(&state.ck, &dh).await?;
                state.ck = self.replace_key(&state.ck, &ck).await?;
                state.k = self.replace_key(&state.k, &k).await?;
                state.h = self.mix_hash(&state.h, c.as_slice());

                // 4. ENCRYPT(k, n++, h, payload)
                //    h = SHA256(concat(h, c))
                let to_send = IdentityAndCredential {
                    identity: EncodedPublicIdentity::from(&self.identity().await?)?,
                    signature: self.sign_static_key(&state.s).await?,
                    credentials: self.credentials().await?,
                };
                let payload = &serde_bare::to_vec(&to_send)?;

                let mut output = message_to_send;
                output.extend_from_slice(payload);

                state.n += 1;
                let c = self
                    .encrypt(&state.k, state.n, &state.h, &output.as_slice())
                    .await?;
                state.h = self.mix_hash(&state.h, c.as_slice());

                // 5. k1, k2 = HKDF(ck, zerolen, 2)
                let (k1, k2) = self.hkdf(&state.ck, &dh).await?;

                // 6. verify their signature
                let their_identity = self
                    .decode_identity(identity_and_credential.identity)
                    .await?;
                let their_signature = identity_and_credential.signature;
                let their_credentials = identity_and_credential.credentials;
                self.verify_signature(&their_identity, &their_signature, &rs)
                    .await?;
                self.verify_credentials(&their_identity, their_credentials)
                    .await?;

                state.status = Ready {
                    their_identity,
                    keys: CompletedKeyExchange::new(state.h, k2, k1),
                };
                self.handshake.state = state;
                Ok(SendMessage(c))
            }
            // incorrect state / event
            (s, e) => Err(Error::new(
                Origin::Channel,
                Kind::Invalid,
                format!(
                    "Unexpected combination of initiator state and event {:?}/{:?}",
                    s, e
                ),
            )),
        }
    }

    fn get_final_state(&self) -> Option<(Identity, CompletedKeyExchange)> {
        match self.handshake.state.status.clone() {
            Ready {
                their_identity,
                keys,
            } => Some((their_identity, keys)),
            _ => None,
        }
    }
}

pub(super) struct InitiatorStateMachine {
    pub(super) handshake: Handshake<InitiatorStatus>,
}

impl InitiatorStateMachine {
    delegate! {
        to self.handshake {
            async fn generate_static_key(&self) -> Result<KeyId>;
            async fn generate_ephemeral_key(&self) -> Result<KeyId>;
            async fn create_ephemeral_secret(&self, content: Vec<u8>) -> Result<KeyId>;
            async fn get_ephemeral_public_key(&self, key_id: &KeyId) -> Result<PublicKey>;
            async fn generate_diffie_hellman_key( &self, key_id: &KeyId, public_key: &PublicKey) -> Result<KeyId>;
            async fn hkdf(&self, ck: &KeyId, dh: &KeyId) -> Result<(KeyId, KeyId)>;
            async fn replace_key(&self, old_key_id: &KeyId, new_key_id: &KeyId) -> Result<KeyId>;
            fn read_from_message<'a>(&self, message: &'a Vec<u8>, start: usize, end: usize) -> Result<&'a [u8]>;
            fn read_end_of_message<'a>(&'a self, message: &'a Vec<u8>, start: usize) -> Result<&'a [u8]>;
            async fn decrypt(&self, k: &KeyId, n: usize, h: &[u8], c: &[u8]) -> Result<Vec<u8>>;
            async fn encrypt(&self, k: &KeyId, n: usize, h: &[u8], c: &[u8]) -> Result<Vec<u8>> ;
            fn get_protocol_name(&self) -> &'static [u8; 32];
            fn mix_hash(&self, h: &[u8; 32], data: &[u8]) -> [u8; 32];
            async fn identity(&self) -> Result<Identity>;
            async fn credentials(&self) -> Result<Vec<Credential>>;
            async fn sign_static_key(&self, key_id: &KeyId) -> Result<Signature>;
            async fn decode_identity(&self, encoded: EncodedPublicIdentity) -> Result<Identity>;
            async fn verify_signature(&self, their_identity: &Identity, their_signature: &Signature, their_public_key: &PublicKey) -> Result<()>;
            async fn verify_credentials(&self, their_identity: &Identity, credentials: Vec<Credential>) -> Result<()>;
        }
    }
}

impl InitiatorStateMachine {
    pub fn new(
        vault: Arc<dyn XXVault>,
        identities: Arc<Identities>,
        identity: Identity,
        credentials: Vec<Credential>,
        trust_policy: Arc<dyn TrustPolicy>,
        trust_context: Option<TrustContext>,
    ) -> InitiatorStateMachine {
        InitiatorStateMachine {
            handshake: Handshake::new(
                vault,
                identities,
                identity,
                credentials,
                trust_policy,
                trust_context,
                Initial,
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub enum InitiatorStatus {
    Initial,
    WaitingForMessage2,
    Ready {
        their_identity: Identity,
        keys: CompletedKeyExchange,
    },
}
