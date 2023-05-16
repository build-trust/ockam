use crate::secure_channel::handshake::handshake_state_machine::{
    EncodedPublicIdentity, IdentityAndCredential,
};
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
use ockam_core::{Error, Result};
use ockam_key_exchange_xx::{
    XXError, XXVault, AES_GCM_TAGSIZE_USIZE, SHA256_SIZE_U32, SHA256_SIZE_USIZE,
};
use sha2::{Digest, Sha256};
use tracing::info;

pub struct Handshake<T> {
    vault: Arc<dyn XXVault>,
    identities: Arc<Identities>,
    trust_policy: Arc<dyn TrustPolicy>,
    trust_context: Option<TrustContext>,
    credentials_expected: bool,
    pub(super) state: State<T>,
}

impl<T: Clone> Handshake<T> {
    pub(super) async fn initialize(&mut self) -> Result<()> {
        let mut state = self.state.clone();

        // 3. Set k to empty, Set n to 0
        state.k = "".into();
        state.n = 0;

        // 4. Set h and ck to protocol name
        // prologue is empty
        state.h = self.get_protocol_name().clone();
        state.ck = self
            .create_ephemeral_secret(self.get_protocol_name().to_vec())
            .await?;

        // 5. h = SHA256(concat(h, prologue))
        state.mix_hash(state.prologue.clone().as_slice());
        Ok(self.state = state)
    }

    pub(super) async fn encode_message1(&mut self) -> Result<Vec<u8>> {
        let mut state = self.state.clone();
        // 1. h = SHA256(concat(h, e.pubKey))
        //    writeToMessage(bigendian(e.pubKey)
        let ephemeral_public_key = self.get_ephemeral_public_key(&state.e).await?;
        let output = ephemeral_public_key.data().to_vec();
        state.mix_hash(ephemeral_public_key.data());

        // 2. payload = empty string
        //    h = SHA256(concat(h, payload))
        state.mix_hash(&[]);
        self.state = state;
        Ok(output)
    }

    pub(super) async fn decode_message1(&mut self, message: Vec<u8>) -> Result<()> {
        let mut state = self.state.clone();
        // 1. re = readFromMessage(32 bytes)
        //    h = SHA256(concat(h, re))
        self.state.re = PublicKey::new(
            Self::read_from_message(&message, 0, CURVE25519_PUBLIC_LENGTH_USIZE)?.to_vec(),
            SecretType::X25519,
        );
        state.mix_hash(&self.state.re.data());

        // 2. payload = readRemainingMessage()
        //    h = SHA256(concat(h, payload))
        let payload = Self::read_end_of_message(&message, CURVE25519_PUBLIC_LENGTH_USIZE)?;
        state.mix_hash(payload);
        Ok(self.state = state)
    }

    pub(super) async fn encode_message2(&mut self) -> Result<Vec<u8>> {
        let mut state = self.state.clone();
        // 1. h = SHA256(concat(h, e.pubKey))
        //    writeToMessage(bigendian(e.pubKey))
        let ephemeral_public_key = self.get_ephemeral_public_key(&state.e).await?;
        let mut output = ephemeral_public_key.data().to_vec();

        // 2. ck, k = HKDF(ck, DH(e, re), 2)
        //    n = 0
        let dh = self
            .generate_diffie_hellman_key(&state.e, &state.re)
            .await?;
        let (ck, k) = self.hkdf(&state.ck, Some(&dh)).await?;
        state.ck = self.replace_key(&state.ck, &ck).await?;
        state.k = self.replace_key(&state.k, &k).await?;
        state.n = 0;

        // 3. c = ENCRYPT(k, n++, h, s.pubKey)
        //    h = SHA256(concat(h, c))
        //    writeToMessage(bigendian(c))
        state.n += 1;
        let c = self
            .encrypt(&state.k, state.n, &state.h, &output.as_slice())
            .await?;
        state.mix_hash(c.as_slice());
        output.extend_from_slice(c.as_slice());

        // 4. ck, k = HKDF(ck, DH(s, re), 2)
        //     n = 0
        let dh = self
            .generate_diffie_hellman_key(&state.s, &state.re)
            .await?;
        let (ck, k) = self.hkdf(&state.ck, Some(&dh)).await?;
        state.ck = self.replace_key(&state.ck, &ck).await?;
        state.k = self.replace_key(&state.k, &k).await?;
        state.n = 0;

        // 7. c = ENCRYPT(k, n++, h, payload)
        //    h = SHA256(concat(h, c))
        //    writeToMessage(bigendian(c))

        state.n += 1;
        let c = self
            .encrypt(
                &state.k,
                state.n,
                &state.h,
                &state.identity_payload.as_slice(),
            )
            .await?;
        state.mix_hash(c.as_slice());
        output.extend_from_slice(c.as_slice());
        self.state = state;
        Ok(output)
    }

    pub(super) async fn decode_message2(
        &mut self,
        message: Vec<u8>,
    ) -> Result<IdentityAndCredential> {
        let mut state = self.state.clone();
        // 1. re = readFromMessage(32 bytes)
        //    h = SHA256(concat(h, re))
        self.state.re = PublicKey::new(
            Self::read_from_message(&message, 0, CURVE25519_PUBLIC_LENGTH_USIZE)?.to_vec(),
            SecretType::X25519,
        );
        state.mix_hash(&self.state.re.data());

        // 2. ck, k = HKDF(ck, DH(e, re), 2)
        // n = 0
        let dh = self
            .generate_diffie_hellman_key(&state.e, &state.re)
            .await?;
        let (ck, k) = self.hkdf(&state.ck, Some(&dh)).await?;
        state.ck = self.replace_key(&state.ck, &ck).await?;
        state.k = self.replace_key(&state.k, &k).await?;
        state.n = 0;

        // 3. c = readFromMessage(48 bytes)
        //    h = SHA256(concat(h, c))
        let c = Self::read_from_message(
            &message,
            CURVE25519_PUBLIC_LENGTH_USIZE,
            CURVE25519_PUBLIC_LENGTH_USIZE + AES_GCM_TAGSIZE_USIZE,
        )?;
        state.mix_hash(c);

        // 4. rs = DECRYPT(k, n++, h, c)
        state.n += 1;
        state.rs = PublicKey::new(
            self.decrypt(&state.k, state.n, &state.h, c).await?,
            SecretType::X25519,
        );

        // 5. ck, k = HKDF(ck, DH(e, rs), 2)
        let dh = self
            .generate_diffie_hellman_key(&state.e, &state.rs)
            .await?;
        let (ck, k) = self.hkdf(&state.ck, Some(&dh)).await?;
        state.ck = self.replace_key(&state.ck, &ck).await?;
        state.k = self.replace_key(&state.k, &k).await?;

        // 6. c = readRemainingMessage()
        //    h = SHA256(concat(h, c))
        let c = Self::read_end_of_message(
            &message,
            CURVE25519_PUBLIC_LENGTH_USIZE * 2 + AES_GCM_TAGSIZE_USIZE,
        )?;
        state.mix_hash(c);

        // 7. payload = DECRYPT(k, n++, h, c)
        state.n += 1;

        let payload = self.decrypt(&state.k, state.n, &state.h, c).await?;
        self.state = state;
        serde_bare::from_slice(payload.as_slice())
            .map_err(|error| Error::new(Origin::Channel, Kind::Invalid, error))
    }

    pub(super) async fn encode_message3(&mut self) -> Result<Vec<u8>> {
        let mut state = self.state.clone();
        // 1. ENCRYPT(k, n++, h, s.pubKey)
        //    h = SHA256(concat(h, c))
        state.n += 1;
        let s_public_key = self.get_ephemeral_public_key(&state.s).await?;
        let c = self
            .encrypt(&state.k, state.n, &state.h, &s_public_key.data())
            .await?;
        state.mix_hash(c.as_slice());

        // 2. writeToMessage(bigendian(c))
        let message_to_send = c.to_vec();

        // 3. ck, k = HKDF(ck, DH(s, re), 2)
        //    h = SHA256(concat(h, c))
        let dh = self
            .generate_diffie_hellman_key(&state.s, &state.re)
            .await?;
        let (ck, k) = self.hkdf(&state.ck, Some(&dh)).await?;
        state.ck = self.replace_key(&state.ck, &ck).await?;
        state.k = self.replace_key(&state.k, &k).await?;

        // 4. ENCRYPT(k, n++, h, payload)
        //    h = SHA256(concat(h, c))
        let mut output = message_to_send;
        output.extend_from_slice(state.identity_payload.as_slice());

        state.n += 1;
        let c = self
            .encrypt(&state.k, state.n, &state.h, &output.as_slice())
            .await?;
        state.mix_hash(c.as_slice());
        self.state = state;
        Ok(output)
    }

    pub(super) async fn decode_message3(
        &mut self,
        message: Vec<u8>,
    ) -> Result<IdentityAndCredential> {
        let mut state = self.state.clone();
        // 1. c = readFromMessage(48 bytes)
        //    h = SHA256(concat(h, c))
        //    rs = DECRYPT(k, n++, h, c)
        let c = Self::read_from_message(
            &message,
            0,
            CURVE25519_PUBLIC_LENGTH_USIZE + AES_GCM_TAGSIZE_USIZE,
        )?;
        state.n += 1;
        state.rs = PublicKey::new(
            self.decrypt(&state.k, state.n, &state.h, c).await?,
            SecretType::X25519,
        );

        // 2. ck, k = HKDF(ck, DH(e, rs), 2)
        //    n = 0
        let dh = self
            .generate_diffie_hellman_key(&state.e, &state.rs)
            .await?;
        let (ck, k) = self.hkdf(&state.ck, Some(&dh)).await?;
        state.ck = self.replace_key(&state.ck, &ck).await?;
        state.k = self.replace_key(&state.k, &k).await?;
        state.n = 0;

        // 3. c = readRemainingMessage()
        //    h = SHA256(concat(h, c))
        let c = Self::read_end_of_message(
            &message,
            CURVE25519_PUBLIC_LENGTH_USIZE * 2 + AES_GCM_TAGSIZE_USIZE,
        )?;
        state.mix_hash(c);

        // 4. payload = DECRYPT(k, n++, h, c)
        state.n += 1;
        let payload = self.decrypt(&state.k, state.n, &state.h, c).await?;
        self.state = state;
        serde_bare::from_slice(payload.as_slice())
            .map_err(|error| Error::new(Origin::Channel, Kind::Invalid, error))
    }

    pub(super) async fn verify_identity(
        &self,
        identity_and_credential: IdentityAndCredential,
    ) -> Result<Identity> {
        let their_identity = self
            .decode_identity(identity_and_credential.identity)
            .await?;
        let their_signature = identity_and_credential.signature;
        let their_credentials = identity_and_credential.credentials;
        self.verify_signature(&their_identity, &their_signature, &self.state.rs)
            .await?;
        self.verify_credentials(&their_identity, their_credentials)
            .await?;
        Ok(their_identity)
    }
}

impl<T> Handshake<T> {
    pub(super) async fn new(
        vault: Arc<dyn XXVault>,
        identities: Arc<Identities>,
        identity: Identity,
        credentials: Vec<Credential>,
        trust_policy: Arc<dyn TrustPolicy>,
        trust_context: Option<TrustContext>,
        status: T,
    ) -> Result<Handshake<T>> {
        // 1. generate a static key pair for this handshake and set it to s
        let s = Self::generate_static_key(vault.clone()).await?;

        // 2. generate an ephemeral key pair for this handshake and set it to e
        let e = Self::generate_ephemeral_key(vault.clone()).await?;

        // 3. prepare the payload that will be sent either in message 2 or message 3
        let payload = IdentityAndCredential {
            identity: EncodedPublicIdentity::from(&identity)?,
            signature: Self::sign_static_key(vault.clone(), identities.clone(), identity, &s)
                .await?,
            credentials: credentials.clone(),
        };
        let identity_payload = serde_bare::to_vec(&payload)?;

        Ok(Handshake {
            vault,
            identities,
            trust_policy,
            trust_context,
            credentials_expected: !credentials.is_empty(),
            state: State::new(status, s, e, identity_payload),
        })
    }

    async fn sign_static_key(
        vault: Arc<dyn XXVault>,
        identities: Arc<Identities>,
        identity: Identity,
        key_id: &KeyId,
    ) -> Result<Signature> {
        let public_static_key = vault.secret_public_key_get(key_id).await?;
        identities
            .identities_keys()
            .create_signature(&identity, public_static_key.data(), None)
            .await
    }

    async fn generate_static_key(vault: Arc<dyn XXVault>) -> Result<KeyId> {
        let attributes = SecretAttributes::new(
            SecretType::X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        );
        vault.secret_generate(attributes).await
    }

    async fn generate_ephemeral_key(vault: Arc<dyn XXVault>) -> Result<KeyId> {
        let attributes = SecretAttributes::new(
            SecretType::X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        );
        vault.secret_generate(attributes).await
    }

    pub(super) async fn create_ephemeral_secret(&self, content: Vec<u8>) -> Result<KeyId> {
        self.vault
            .secret_import(Secret::Key(SecretKey::new(content)), Self::ck_attributes())
            .await
    }

    pub(super) async fn get_ephemeral_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.vault.secret_public_key_get(key_id).await
    }

    pub(super) async fn generate_diffie_hellman_key(
        &self,
        key_id: &KeyId,
        public_key: &PublicKey,
    ) -> Result<KeyId> {
        self.vault.ec_diffie_hellman(key_id, public_key).await
    }

    pub(super) async fn hkdf(&self, ck: &KeyId, dh: Option<&KeyId>) -> Result<(KeyId, KeyId)> {
        let mut hkdf_output = self
            .vault
            .hkdf_sha256(
                ck,
                b"",
                dh,
                vec![Self::ck_attributes(), Self::k_attributes()],
            )
            .await?;

        if hkdf_output.len() != 2 {
            return Err(XXError::InternalVaultError.into());
        }

        let ck = hkdf_output.pop().unwrap();
        let k = hkdf_output.pop().unwrap();

        Ok((ck, k))
    }

    pub(super) async fn replace_key(
        &self,
        old_key_id: &KeyId,
        new_key_id: &KeyId,
    ) -> Result<KeyId> {
        self.vault.secret_destroy(old_key_id.clone()).await?;
        Ok(new_key_id.clone())
    }

    pub(super) fn read_from_message(message: &Vec<u8>, start: usize, end: usize) -> Result<&[u8]> {
        if message.len() < end || start > end {
            return Err(XXError::MessageLenMismatch.into());
        }
        Ok(&message[start..end])
    }

    pub(super) fn read_end_of_message(message: &Vec<u8>, start: usize) -> Result<&[u8]> {
        if message.len() < start {
            return Err(XXError::MessageLenMismatch.into());
        }
        Ok(&message[start..])
    }

    pub(super) async fn decrypt(&self, k: &KeyId, n: usize, h: &[u8], c: &[u8]) -> Result<Vec<u8>> {
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&n.to_be_bytes());
        self.vault
            .aead_aes_gcm_decrypt(k, c, nonce.as_ref(), h)
            .await
            .map(|b| b.to_vec())
    }

    pub(super) async fn encrypt(&self, k: &KeyId, n: usize, h: &[u8], c: &[u8]) -> Result<Vec<u8>> {
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&n.to_be_bytes());
        self.vault
            .aead_aes_gcm_encrypt(k, c, nonce.as_ref(), h)
            .await
            .map(|b| b.to_vec())
    }

    pub(super) fn get_protocol_name(&self) -> &'static [u8; 32] {
        b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0"
    }

    pub(super) async fn decode_identity(&self, encoded: EncodedPublicIdentity) -> Result<Identity> {
        self.identities
            .identities_creation()
            .import_identity(&encoded.encoded)
            .await
    }

    pub(super) async fn verify_signature(
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

    pub(super) async fn verify_credentials(
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
        } else if self.credentials_expected {
            // we cannot validate credentials without a trust context
            return Err(IdentityError::SecureChannelVerificationFailed.into());
        };
        Ok(())
    }

    fn ck_attributes() -> SecretAttributes {
        SecretAttributes::new(
            SecretType::Buffer,
            SecretPersistence::Ephemeral,
            SHA256_SIZE_U32,
        )
    }

    fn k_attributes() -> SecretAttributes {
        SecretAttributes::new(
            SecretType::Aes,
            SecretPersistence::Ephemeral,
            AES256_SECRET_LENGTH_U32,
        )
    }

    fn identities(&self) -> Arc<Identities> {
        self.identities.clone()
    }

    fn identities_keys(&self) -> Arc<IdentitiesKeys> {
        self.identities.identities_keys()
    }
}

#[derive(Debug, Clone)]
pub(super) struct State<T> {
    pub(super) s: KeyId,
    pub(super) e: KeyId,
    pub(super) k: KeyId,
    pub(super) re: PublicKey,
    pub(super) rs: PublicKey,
    pub(super) n: usize,
    pub(super) h: [u8; SHA256_SIZE_USIZE],
    pub(super) ck: KeyId,
    pub(super) prologue: Vec<u8>,
    pub(super) identity_payload: Vec<u8>,
    pub(super) status: T,
}

impl<T> State<T> {
    pub(super) fn new(status: T, s: KeyId, e: KeyId, identity_payload: Vec<u8>) -> State<T> {
        State {
            s,
            e,
            k: "".to_string(),
            re: PublicKey::new(vec![], SecretType::X25519),
            rs: PublicKey::new(vec![], SecretType::X25519),
            n: 0,
            h: [0u8; SHA256_SIZE_USIZE],
            ck: "".to_string(),
            prologue: vec![],
            identity_payload,
            status,
        }
    }

    pub(super) fn mix_hash(&mut self, data: &[u8]) {
        let mut input = self.h.to_vec();
        input.extend_from_slice(data.as_ref());
        self.h = Self::sha256(&input)
    }

    fn sha256(data: &[u8]) -> [u8; 32] {
        let digest = Sha256::digest(data);
        *array_ref![digest, 0, 32]
    }
}
