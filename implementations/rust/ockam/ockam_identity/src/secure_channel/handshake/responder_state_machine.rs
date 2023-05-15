use crate::secure_channel::handshake::handshake_state::Handshake;
use crate::secure_channel::handshake::handshake_state_machine::Action::NoAction;
use crate::secure_channel::handshake::handshake_state_machine::Event::ReceivedMessage;
use crate::secure_channel::handshake::handshake_state_machine::{
    Action, EncodedPublicIdentity, Event, IdentityAndCredential, StateMachine,
};
use crate::{Credential, Identities, Identity, TrustContext, TrustPolicy};
use async_trait::async_trait;
use delegate::delegate;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::{KeyId, PublicKey, SecretType, Signature, CURVE25519_PUBLIC_LENGTH_USIZE};
use ockam_core::{CompletedKeyExchange, Error, Result};
use ockam_key_exchange_xx::{XXVault, AES_GCM_TAGSIZE_USIZE};
use Action::*;
use Event::*;
use ResponderStatus::*;

#[async_trait]
impl StateMachine for ResponderStateMachine {
    async fn on_event(&mut self, event: Event) -> Result<Action> {
        let mut state = self.state_machine.state.clone();
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

                // Set the new status and return the action
                state.status = WaitingForMessage1;
                self.state_machine.state = state;
                Ok(NoAction)
            }
            //
            (WaitingForMessage1, ReceivedMessage(message)) => {
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

                // 2. payload = readRemainingMessage()
                //    h = SHA256(concat(h, payload))
                let payload = self.read_end_of_message(&message, CURVE25519_PUBLIC_LENGTH_USIZE)?;
                state.h = self.mix_hash(&state.h, payload);

                // -------
                // SEND
                // -------

                // 3. h = SHA256(concat(h, e.pubKey))
                //    writeToMessage(bigendian(e.pubKey))
                let ephemeral_public_key = self.get_ephemeral_public_key(&state.e).await?;
                let mut output = ephemeral_public_key.data().to_vec();

                // 4. ck, k = HKDF(ck, DH(e, re), 2)
                // n = 0
                let dh = self.generate_diffie_hellman_key(&state.e, &re).await?;
                let (ck, k) = self.hkdf(&state.ck, &dh).await?;
                state.ck = self.replace_key(&state.ck, &ck).await?;
                state.k = self.replace_key(&state.k, &k).await?;
                state.n = 0;

                // 5. c = ENCRYPT(k, n++, h, s.pubKey)
                //    h = SHA256(concat(h, c))
                //    writeToMessage(bigendian(c))
                state.n += 1;
                let c = self
                    .encrypt(&state.k, state.n, &state.h, &output.as_slice())
                    .await?;
                state.h = self.mix_hash(&state.h, c.as_slice());
                output.extend_from_slice(c.as_slice());

                // 6. ck, k = HKDF(ck, DH(s, re), 2)
                // n = 0
                let dh = self.generate_diffie_hellman_key(&state.s, &re).await?;
                let (ck, k) = self.hkdf(&state.ck, &dh).await?;
                state.ck = self.replace_key(&state.ck, &ck).await?;
                state.k = self.replace_key(&state.k, &k).await?;
                state.n = 0;

                // 7. c = ENCRYPT(k, n++, h, payload)
                //    h = SHA256(concat(h, c))
                //    writeToMessage(bigendian(c))
                let to_send = IdentityAndCredential {
                    identity: EncodedPublicIdentity::from(&self.identity().await?)?,
                    signature: self.sign_static_key(&state.s).await?,
                    credentials: self.credentials().await?,
                };
                let payload = &serde_bare::to_vec(&to_send)?;

                state.n += 1;
                let c = self
                    .encrypt(&state.k, state.n, &state.h, &payload.as_slice())
                    .await?;
                state.h = self.mix_hash(&state.h, c.as_slice());
                output.extend_from_slice(c.as_slice());

                // Set the new status and return the action
                state.status = WaitingForMessage3;
                self.state_machine.state = state;
                Ok(SendMessage(output))
            }
            //
            (WaitingForMessage3, ReceivedMessage(message)) => {
                // 1. c = readFromMessage(48 bytes)
                //    h = SHA256(concat(h, c))
                //    rs = DECRYPT(k, n++, h, c)
                let c = self.read_from_message(
                    &message,
                    0,
                    CURVE25519_PUBLIC_LENGTH_USIZE + AES_GCM_TAGSIZE_USIZE,
                )?;
                state.n += 1;
                let rs = PublicKey::new(
                    self.decrypt(&state.k, state.n, &state.h, c).await?,
                    SecretType::X25519,
                );

                // 2. ck, k = HKDF(ck, DH(e, rs), 2)
                // n = 0
                let dh = self.generate_diffie_hellman_key(&state.e, &rs).await?;
                let (ck, k) = self.hkdf(&state.ck, &dh).await?;
                state.ck = self.replace_key(&state.ck, &ck).await?;
                state.k = self.replace_key(&state.k, &k).await?;
                state.n = 0;

                // 3. c = readRemainingMessage()
                //    h = SHA256(concat(h, c))
                let c = self.read_end_of_message(
                    &message,
                    CURVE25519_PUBLIC_LENGTH_USIZE * 2 + AES_GCM_TAGSIZE_USIZE,
                )?;
                state.h = self.mix_hash(&state.h, c);

                // 4. payload = DECRYPT(k, n++, h, c)
                state.n += 1;
                let payload = self.decrypt(&state.k, state.n, &state.h, c).await?;
                let identity_and_credential: IdentityAndCredential =
                    serde_bare::from_slice(payload.as_slice())
                        .map_err(|error| Error::new(Origin::Channel, Kind::Invalid, error))?;

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
                    keys: CompletedKeyExchange::new(state.h, k1, k2),
                };
                self.state_machine.state = state;
                Ok(NoAction)
            }
            // incorrect state / event
            (s, e) => Err(Error::new(
                Origin::Channel,
                Kind::Invalid,
                format!(
                    "Unexpected combination of responder state and event {:?}/{:?}",
                    s, e
                ),
            )),
        }
    }

    fn get_final_state(&self) -> Option<(Identity, CompletedKeyExchange)> {
        match self.state_machine.state.status.clone() {
            Ready {
                their_identity,
                keys,
            } => Some((their_identity, keys)),
            _ => None,
        }
    }
}

pub struct ResponderStateMachine {
    state_machine: Handshake<ResponderStatus>,
}

impl ResponderStateMachine {
    delegate! {
        to self.state_machine {
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

impl ResponderStateMachine {
    pub fn new(
        vault: Arc<dyn XXVault>,
        identities: Arc<Identities>,
        identity: Identity,
        credentials: Vec<Credential>,
        trust_policy: Arc<dyn TrustPolicy>,
        trust_context: Option<TrustContext>,
    ) -> ResponderStateMachine {
        ResponderStateMachine {
            state_machine: Handshake::new(
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
pub enum ResponderStatus {
    Initial,
    WaitingForMessage1,
    WaitingForMessage3,
    Ready {
        their_identity: Identity,
        keys: CompletedKeyExchange,
    },
}
