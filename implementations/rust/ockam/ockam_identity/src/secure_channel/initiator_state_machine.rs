use crate::secure_channel::packets::{EncodedPublicIdentity, IdentityAndCredential};
use crate::{
    Credential, Credentials, Identities, IdentitiesKeys, Identity, IdentityError,
    SecureChannelTrustInfo, TrustContext, TrustPolicy,
};
use arrayref::array_ref;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::{
    KeyId, PublicKey, Secret, SecretAttributes, SecretKey, SecretPersistence, SecretType,
    Signature, AES256_SECRET_LENGTH_U32, CURVE25519_PUBLIC_LENGTH_USIZE,
    CURVE25519_SECRET_LENGTH_U32,
};
use ockam_core::{CompletedKeyExchange, Error, Result};
use ockam_key_exchange_xx::{
    XXError, XXVault, AES_GCM_TAGSIZE_USIZE, SHA256_SIZE_U32, SHA256_SIZE_USIZE,
};
use sha2::{Digest, Sha256};
use tracing::info;
use Action::*;
use Event::*;
use InitiatorStatus::*;

pub struct InitiatorStateMachine {
    vault: Arc<dyn XXVault>,
    identities: Arc<Identities>,
    identity: Identity,
    credentials: Vec<Credential>,
    trust_policy: Arc<dyn TrustPolicy>,
    trust_context: Option<TrustContext>,
    pub(crate) state: InitiatorState,
}

impl InitiatorStateMachine {
    pub async fn on_event(&mut self, event: Event) -> Result<Action> {
        let mut state = self.state.clone();
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
                let ephemeral_public_key = self.get_ephemeral_public_key(&state.e).await?;
                state.h = self.mix_hash(&state.h, ephemeral_public_key.data());

                // 7. h = SHA256(concat(h, payload))
                let payload = &[];
                state.h = self.mix_hash(&state.h, payload.as_ref());

                let mut output = ephemeral_public_key.data().to_vec();
                output.extend_from_slice(payload);

                // Set the new status and return the action
                state.status = WaitingForMessage;
                self.state = state;
                Ok(SendMessage(output))
            }
            //
            (WaitingForMessage, ReceivedMessage(message)) => {
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
                self.state = state;
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
}

impl InitiatorStateMachine {
    async fn generate_static_key(&self) -> Result<KeyId> {
        let attributes = SecretAttributes::new(
            SecretType::X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        );
        self.vault.secret_generate(attributes).await
    }

    async fn generate_ephemeral_key(&self) -> Result<KeyId> {
        let attributes = SecretAttributes::new(
            SecretType::X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        );
        self.vault.secret_generate(attributes).await
    }

    async fn create_ephemeral_secret(&self, content: Vec<u8>) -> Result<KeyId> {
        self.vault
            .secret_import(Secret::Key(SecretKey::new(content)), self.ck_attributes())
            .await
    }

    async fn get_ephemeral_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.vault.secret_public_key_get(key_id).await
    }

    async fn generate_diffie_hellman_key(
        &self,
        key_id: &KeyId,
        public_key: &PublicKey,
    ) -> Result<KeyId> {
        self.vault.ec_diffie_hellman(key_id, public_key).await
    }

    async fn hkdf(&self, ck: &KeyId, dh: &KeyId) -> Result<(KeyId, KeyId)> {
        let mut hkdf_output = self
            .vault
            .hkdf_sha256(
                ck,
                b"",
                Some(dh),
                vec![self.ck_attributes(), self.k_attributes()],
            )
            .await?;

        if hkdf_output.len() != 2 {
            return Err(XXError::InternalVaultError.into());
        }

        let ck = hkdf_output.pop().unwrap();
        let k = hkdf_output.pop().unwrap();

        Ok((ck, k))
    }

    async fn replace_key(&self, old_key_id: &KeyId, new_key_id: &KeyId) -> Result<KeyId> {
        self.vault.secret_destroy(old_key_id.clone()).await?;
        Ok(new_key_id.clone())
    }

    fn read_from_message<'a>(
        &self,
        message: &'a Vec<u8>,
        start: usize,
        end: usize,
    ) -> Result<&'a [u8]> {
        if message.len() < end || start > end {
            return Err(XXError::MessageLenMismatch.into());
        }
        Ok(&message[start..end])
    }

    fn read_end_of_message<'a>(&'a self, message: &'a Vec<u8>, start: usize) -> Result<&'a [u8]> {
        if message.len() < start {
            return Err(XXError::MessageLenMismatch.into());
        }
        Ok(&message[start..])
    }

    async fn decrypt(&self, k: &KeyId, n: usize, h: &[u8], c: &[u8]) -> Result<Vec<u8>> {
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&n.to_be_bytes());
        self.vault
            .aead_aes_gcm_decrypt(k, c, nonce.as_ref(), h)
            .await
            .map(|b| b.to_vec())
    }

    async fn encrypt(&self, k: &KeyId, n: usize, h: &[u8], c: &[u8]) -> Result<Vec<u8>> {
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&n.to_be_bytes());
        self.vault
            .aead_aes_gcm_encrypt(k, c, nonce.as_ref(), h)
            .await
            .map(|b| b.to_vec())
    }

    fn get_protocol_name(&self) -> &'static [u8; 32] {
        b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0"
    }

    fn sha256(&self, data: &[u8]) -> [u8; 32] {
        let digest = Sha256::digest(data);
        *array_ref![digest, 0, 32]
    }

    fn mix_hash(&self, h: &[u8; 32], data: &[u8]) -> [u8; 32] {
        let mut input = h.to_vec();
        input.extend_from_slice(data.as_ref());
        self.sha256(&input)
    }

    async fn identity(&self) -> Result<Identity> {
        Ok(self.identity.clone())
    }

    async fn credentials(&self) -> Result<Vec<Credential>> {
        Ok(self.credentials.clone())
    }

    async fn sign_static_key(&self, key_id: &KeyId) -> Result<Signature> {
        let public_static_key = self.vault.secret_public_key_get(key_id).await?;
        self.identities_keys()
            .create_signature(&self.identity().await?, public_static_key.data(), None)
            .await
    }

    async fn decode_identity(&self, encoded: EncodedPublicIdentity) -> Result<Identity> {
        self.identities
            .identities_creation()
            .import_identity(&encoded.encoded)
            .await
    }

    async fn verify_signature(
        &self,
        their_identity: &Identity,
        their_signature: &Signature,
        their_public_key: &PublicKey,
    ) -> Result<()> {
        //verify the signature of the static key used during noise exchanges
        //actually matches the signature of the identity
        let signature_verified = self
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

    async fn verify_credentials(
        &self,
        their_identity: &Identity,
        credentials: Vec<Credential>,
    ) -> Result<()> {
        // Check our TrustPolicy
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
                    .identities()
                    .receive_presented_credential(
                        &their_identity.identifier,
                        &[trust_context.authority()?.identity()],
                        credential,
                    )
                    .await;

                if let Some(_err) = result.err() {
                    //TODO: consider the possibility of keep going when a credential validation fails
                    return Err(IdentityError::SecureChannelVerificationFailed.into());
                }
            }
        } else if !self.credentials.is_empty() {
            //we cannot validate credentials without a trust context
            return Err(IdentityError::SecureChannelVerificationFailed.into());
        };
        Ok(())
    }

    fn ck_attributes(&self) -> SecretAttributes {
        SecretAttributes::new(
            SecretType::Buffer,
            SecretPersistence::Ephemeral,
            SHA256_SIZE_U32,
        )
    }

    fn k_attributes(&self) -> SecretAttributes {
        SecretAttributes::new(
            SecretType::Aes,
            SecretPersistence::Ephemeral,
            AES256_SECRET_LENGTH_U32,
        )
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
            vault,
            identities,
            identity,
            credentials,
            trust_policy,
            trust_context,
            state: InitiatorState::new(),
        }
    }

    fn identities(&self) -> Arc<Identities> {
        self.identities.clone()
    }

    fn identities_keys(&self) -> Arc<IdentitiesKeys> {
        self.identities.identities_keys()
    }
}

#[derive(Debug, Clone)]
pub struct InitiatorState {
    s: KeyId,
    e: KeyId,
    k: KeyId,
    n: usize,
    h: [u8; SHA256_SIZE_USIZE],
    ck: KeyId,
    prologue: Vec<u8>,
    pub(crate) status: InitiatorStatus,
}

impl InitiatorState {
    fn new() -> InitiatorState {
        InitiatorState {
            s: "".to_string(),
            e: "".to_string(),
            k: "".to_string(),
            n: 0,
            h: [0u8; SHA256_SIZE_USIZE],
            ck: "".to_string(),
            prologue: vec![],
            status: Initial,
        }
    }
}

#[derive(Debug, Clone)]
pub enum InitiatorStatus {
    Initial,
    WaitingForMessage,
    Ready {
        their_identity: Identity,
        keys: CompletedKeyExchange,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Initialize,
    ReceivedMessage(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    SendMessage(Vec<u8>),
}
