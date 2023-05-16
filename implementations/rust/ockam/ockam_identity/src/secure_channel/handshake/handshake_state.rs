use crate::secure_channel::handshake::handshake_state_machine::{
    EncodedPublicIdentity, IdentityAndCredentials,
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
use ockam_core::{CompletedKeyExchange, Error, Result};
use ockam_key_exchange_xx::{
    XXError, XXVault, AES_GCM_TAGSIZE_USIZE, SHA256_SIZE_U32, SHA256_SIZE_USIZE,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tracing::info;
use SecretType::*;
use Status::*;

pub struct Handshake {
    vault: Arc<dyn XXVault>,
    identities: Arc<Identities>,
    trust_policy: Arc<dyn TrustPolicy>,
    trust_context: Option<TrustContext>,
    credentials_expected: bool,
    pub(super) state: State,
}

impl Handshake {
    pub(super) async fn initialize(&mut self) -> Result<()> {
        let mut state = self.state.clone();
        state.h = self.protocol_name().clone();
        state.ck = self.import_secret(self.protocol_name().to_vec()).await?;
        state.mix_hash(state.prologue.clone().as_slice());

        Ok(self.state = state)
    }

    pub(super) async fn encode_message1(&mut self) -> Result<Vec<u8>> {
        let mut state = self.state.clone();
        // output e.pubKey
        let e_pub_key = self.get_public_key(&state.e).await?;
        let mut message = e_pub_key.data().to_vec();
        state.mix_hash(e_pub_key.data());

        // output message 1 payload
        message.extend(state.message1_payload.clone());
        state.mix_hash(state.message1_payload.clone().as_slice());

        self.state = state;
        Ok(message)
    }

    pub(super) async fn decode_message1(&mut self, message: Vec<u8>) -> Result<()> {
        let mut state = self.state.clone();
        // read e.pubKey
        let key = Self::read_key(&message)?;
        state.re = PublicKey::new(key.to_vec(), X25519);
        state.mix_hash(key);

        // decode payload
        let payload = Self::read_message1_payload(&message)?;
        state.mix_hash(payload);
        Ok(self.state = state)
    }

    pub(super) async fn encode_message2(&mut self) -> Result<Vec<u8>> {
        let mut state = self.state.clone();
        // output e.pubKey
        let e_pub_key = self.get_public_key(&state.e).await?;
        let mut message2 = e_pub_key.data().to_vec();

        // ck, k = HKDF(ck, DH(e, re), 2)
        let dh = self.dh(&state.e, &state.re).await?;
        (state.ck, state.k) = self.hkdf(&state.ck, &state.k, Some(&dh)).await?;

        // encrypt and output s.pubKey
        let s_pub_key = self.get_public_key(&state.s).await?;
        let c = self.encrypt(&state.k, &state.h, &s_pub_key.data()).await?;
        message2.extend_from_slice(c.as_slice());
        state.mix_hash(c.as_slice());

        // ck, k = HKDF(ck, DH(s, re), 2)
        let dh = self.dh(&state.s, &state.re).await?;
        (state.ck, state.k) = self.hkdf(&state.ck, &state.k, Some(&dh)).await?;

        // encrypt and output payload
        let c = self
            .encrypt(&state.k, &state.h, &state.identity_payload.as_slice())
            .await?;
        message2.extend(c.clone());
        state.mix_hash(c.as_slice());
        self.state = state;
        Ok(message2)
    }

    pub(super) async fn decode_message2(
        &mut self,
        message: Vec<u8>,
    ) -> Result<IdentityAndCredentials> {
        let mut state = self.state.clone();
        // decode re.pubKey
        let re_pub_key = Self::read_key(&message)?;
        state.re = PublicKey::new(re_pub_key.to_vec(), X25519);
        state.mix_hash(re_pub_key);

        // ck, k = HKDF(ck, DH(e, re), 2)
        let dh = self.dh(&state.e, &state.re).await?;
        (state.ck, state.k) = self.hkdf(&state.ck, &state.k, Some(&dh)).await?;

        // decrypt rs.pubKey
        let rs_pub_key = Self::read_message2_encrypted_key(&message)?;
        state.rs = PublicKey::new(self.decrypt(&state.k, &state.h, rs_pub_key).await?, X25519);
        state.mix_hash(rs_pub_key);

        // ck, k = HKDF(ck, DH(e, rs), 2)
        let dh = self.dh(&state.e, &state.rs).await?;
        (state.ck, state.k) = self.hkdf(&state.ck, &state.k, Some(&dh)).await?;

        // decrypt payload
        let c = Self::read_message2_payload(&message)?;
        let payload = self.decrypt(&state.k, &state.h, c).await?;
        state.mix_hash(c);

        self.state = state;
        Self::deserialize(payload)
    }

    pub(super) async fn encode_message3(&mut self) -> Result<Vec<u8>> {
        let mut state = self.state.clone();
        // encrypt s.pubKey
        let s_pub_key = self.get_public_key(&state.s).await?;
        let c = self.encrypt(&state.k, &state.h, &s_pub_key.data()).await?;
        let mut message3 = c.to_vec();
        state.mix_hash(c.as_slice());

        // ck, k = HKDF(ck, DH(s, re), 2)
        let dh = self.dh(&state.s, &state.re).await?;
        (state.ck, state.k) = self.hkdf(&state.ck, &state.k, Some(&dh)).await?;

        // encrypt payload
        let c = self
            .encrypt(&state.k, &state.h, &state.identity_payload.as_slice())
            .await?;
        message3.extend(c.clone());
        state.mix_hash(c.as_slice());

        self.state = state;
        Ok(message3)
    }

    pub(super) async fn decode_message3(
        &mut self,
        message: Vec<u8>,
    ) -> Result<IdentityAndCredentials> {
        let mut state = self.state.clone();
        // decrypt rs key
        let rs_pub_key = Self::read_message3_encrypted_key(&message)?;
        state.rs = PublicKey::new(self.decrypt(&state.k, &state.h, rs_pub_key).await?, X25519);

        // ck, k = HKDF(ck, DH(e, rs), 2), n = 0
        let dh = self.dh(&state.e, &state.rs).await?;
        (state.ck, state.k) = self.hkdf(&state.ck, &state.k, Some(&dh)).await?;

        // decrypt payload
        let c = Self::read_message3_payload(&message)?;
        let payload = self.decrypt(&state.k, &state.h, c).await?;
        state.mix_hash(c);
        self.state = state;
        Self::deserialize(payload)
    }

    pub(super) async fn verify_identity(&self, peer: IdentityAndCredentials) -> Result<Identity> {
        let identity = self.decode_identity(peer.identity).await?;
        self.verify_signature(&identity, &peer.signature, &self.state.rs)
            .await?;
        self.verify_credentials(&identity, peer.credentials).await?;
        Ok(identity)
    }

    pub(super) fn get_final_state(&self) -> Option<(Identity, CompletedKeyExchange)> {
        match self.state.status.clone() {
            Ready {
                their_identity,
                keys,
            } => Some((their_identity, keys)),
            _ => None,
        }
    }
}

impl Handshake {
    pub(super) async fn new(
        vault: Arc<dyn XXVault>,
        identities: Arc<Identities>,
        identity: Identity,
        credentials: Vec<Credential>,
        trust_policy: Arc<dyn TrustPolicy>,
        trust_context: Option<TrustContext>,
    ) -> Result<Handshake> {
        // 1. generate a static key pair for this handshake and set it to s
        let s = Self::generate_static_key(vault.clone()).await?;

        // 2. generate an ephemeral key pair for this handshake and set it to e
        let e = Self::generate_ephemeral_key(vault.clone()).await?;

        // 3. prepare the payload that will be sent either in message 2 or message 3
        let payload = IdentityAndCredentials {
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
            state: State::new(s, e, identity_payload),
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
            X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        );
        vault.secret_generate(attributes).await
    }

    async fn generate_ephemeral_key(vault: Arc<dyn XXVault>) -> Result<KeyId> {
        let attributes = SecretAttributes::new(
            X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        );
        vault.secret_generate(attributes).await
    }

    fn deserialize<D: for<'a> Deserialize<'a>>(payload: Vec<u8>) -> Result<D> {
        serde_bare::from_slice(payload.as_slice())
            .map_err(|error| Error::new(Origin::Channel, Kind::Invalid, error))
    }

    pub(super) async fn import_secret(&self, content: Vec<u8>) -> Result<KeyId> {
        self.vault
            .secret_import(Secret::Key(SecretKey::new(content)), Self::ck_attributes())
            .await
    }

    pub(super) async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.vault.secret_public_key_get(key_id).await
    }

    pub(super) async fn dh(&self, key_id: &KeyId, public_key: &PublicKey) -> Result<KeyId> {
        self.vault.ec_diffie_hellman(key_id, public_key).await
    }

    pub(super) async fn hkdf(
        &mut self,
        ck: &KeyId,
        k: &KeyId,
        dh: Option<&KeyId>,
    ) -> Result<(KeyId, KeyId)> {
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

        self.vault.secret_destroy(ck.clone()).await?;
        self.vault.secret_destroy(k.clone()).await?;

        let ck = hkdf_output.pop().unwrap();
        let k = hkdf_output.pop().unwrap();
        self.state.n = 0;

        Ok((ck, k))
    }

    pub(super) async fn decrypt(&mut self, k: &KeyId, h: &[u8], c: &[u8]) -> Result<Vec<u8>> {
        self.state.n += 1;
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.state.n.to_be_bytes());
        self.vault
            .aead_aes_gcm_decrypt(k, c, nonce.as_ref(), h)
            .await
            .map(|b| b.to_vec())
    }

    pub(super) async fn encrypt(&mut self, k: &KeyId, h: &[u8], c: &[u8]) -> Result<Vec<u8>> {
        self.state.n += 1;
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.state.n.to_be_bytes());
        self.vault
            .aead_aes_gcm_encrypt(k, c, nonce.as_ref(), h)
            .await
            .map(|b| b.to_vec())
    }

    pub(super) fn protocol_name(&self) -> &'static [u8; 32] {
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

    fn identities(&self) -> Arc<Identities> {
        self.identities.clone()
    }

    fn identities_keys(&self) -> Arc<IdentitiesKeys> {
        self.identities.identities_keys()
    }
}

/// Static functions
impl Handshake {
    fn ck_attributes() -> SecretAttributes {
        SecretAttributes::new(Buffer, SecretPersistence::Ephemeral, SHA256_SIZE_U32)
    }

    fn k_attributes() -> SecretAttributes {
        SecretAttributes::new(Aes, SecretPersistence::Ephemeral, AES256_SECRET_LENGTH_U32)
    }

    fn read_message1_payload(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_end(message, Self::key_size())
    }

    fn read_message2_encrypted_key(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_middle(message, Self::key_size(), Self::encrypted_key_size())
    }

    fn read_message2_payload(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_end(message, Self::key_size() + Self::encrypted_key_size())
    }

    fn read_message3_encrypted_key(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_start(message, Self::encrypted_key_size())
    }

    fn read_message3_payload(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_end(message, Self::encrypted_key_size())
    }

    fn read_start(message: &Vec<u8>, length: usize) -> Result<&[u8]> {
        if message.len() < length {
            return Err(XXError::MessageLenMismatch.into());
        }
        Ok(&message[0..length])
    }

    fn read_end(message: &Vec<u8>, drop_length: usize) -> Result<&[u8]> {
        if message.len() < drop_length {
            return Err(XXError::MessageLenMismatch.into());
        }
        Ok(&message[drop_length..])
    }

    fn read_middle(message: &Vec<u8>, drop_length: usize, length: usize) -> Result<&[u8]> {
        if message.len() < drop_length + length {
            return Err(XXError::MessageLenMismatch.into());
        }
        Ok(&message[drop_length..(drop_length + length)])
    }

    fn read_key(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_start(message, Self::key_size())
    }

    fn key_size() -> usize {
        CURVE25519_PUBLIC_LENGTH_USIZE
    }

    fn encrypted_key_size() -> usize {
        Self::key_size() + AES_GCM_TAGSIZE_USIZE
    }
}

#[derive(Debug, Clone)]
pub(super) struct State {
    pub(super) s: KeyId,
    pub(super) e: KeyId,
    pub(super) k: KeyId,
    pub(super) re: PublicKey,
    pub(super) rs: PublicKey,
    pub(super) n: usize,
    pub(super) h: [u8; SHA256_SIZE_USIZE],
    pub(super) ck: KeyId,
    pub(super) prologue: Vec<u8>,
    pub(super) message1_payload: Vec<u8>,
    pub(super) identity_payload: Vec<u8>,
    pub(super) status: Status,
}

impl State {
    pub(super) fn new(s: KeyId, e: KeyId, identity_payload: Vec<u8>) -> State {
        State {
            s,
            e,
            k: "".to_string(),
            re: PublicKey::new(vec![], X25519),
            rs: PublicKey::new(vec![], X25519),
            n: 0,
            h: [0u8; SHA256_SIZE_USIZE],
            ck: "".to_string(),
            prologue: vec![],
            message1_payload: vec![],
            identity_payload,
            status: Initial,
        }
    }

    pub(super) fn mix_hash(&mut self, data: &[u8]) {
        let mut input = self.h.to_vec();
        input.extend(data);
        self.h = Self::sha256(&input)
    }

    fn sha256(data: &[u8]) -> [u8; 32] {
        let digest = Sha256::digest(data);
        *array_ref![digest, 0, 32]
    }
}

#[derive(Debug, Clone)]
pub(super) enum Status {
    Initial,
    WaitingForMessage1,
    WaitingForMessage2,
    WaitingForMessage3,
    Ready {
        their_identity: Identity,
        keys: CompletedKeyExchange,
    },
}
