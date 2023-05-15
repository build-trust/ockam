use crate::secure_channel::handshake::handshake_state_machine::EncodedPublicIdentity;
use crate::{
    Credential, Credentials, Identities, IdentitiesKeys, Identity, IdentityError,
    SecureChannelTrustInfo, TrustContext, TrustPolicy,
};
use arrayref::array_ref;
use ockam_core::compat::sync::Arc;
use ockam_core::vault::{
    KeyId, PublicKey, Secret, SecretAttributes, SecretKey, SecretPersistence, SecretType,
    Signature, AES256_SECRET_LENGTH_U32, CURVE25519_SECRET_LENGTH_U32,
};
use ockam_core::Result;
use ockam_key_exchange_xx::{XXError, XXVault, SHA256_SIZE_U32, SHA256_SIZE_USIZE};
use sha2::{Digest, Sha256};
use tracing::info;

pub struct Handshake<T> {
    vault: Arc<dyn XXVault>,
    identities: Arc<Identities>,
    identity: Identity,
    credentials: Vec<Credential>,
    trust_policy: Arc<dyn TrustPolicy>,
    trust_context: Option<TrustContext>,
    pub(super) state: State<T>,
}

impl<T> Handshake<T> {
    pub(super) fn new(
        vault: Arc<dyn XXVault>,
        identities: Arc<Identities>,
        identity: Identity,
        credentials: Vec<Credential>,
        trust_policy: Arc<dyn TrustPolicy>,
        trust_context: Option<TrustContext>,
        status: T,
    ) -> Handshake<T> {
        Handshake {
            vault,
            identities,
            identity,
            credentials,
            trust_policy,
            trust_context,
            state: State::new(status),
        }
    }

    pub(super) async fn generate_static_key(&self) -> Result<KeyId> {
        let attributes = SecretAttributes::new(
            SecretType::X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        );
        self.vault.secret_generate(attributes).await
    }

    pub(super) async fn generate_ephemeral_key(&self) -> Result<KeyId> {
        let attributes = SecretAttributes::new(
            SecretType::X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        );
        self.vault.secret_generate(attributes).await
    }

    pub(super) async fn create_ephemeral_secret(&self, content: Vec<u8>) -> Result<KeyId> {
        self.vault
            .secret_import(Secret::Key(SecretKey::new(content)), self.ck_attributes())
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

    pub(super) async fn hkdf(&self, ck: &KeyId, dh: &KeyId) -> Result<(KeyId, KeyId)> {
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

    pub(super) async fn replace_key(
        &self,
        old_key_id: &KeyId,
        new_key_id: &KeyId,
    ) -> Result<KeyId> {
        self.vault.secret_destroy(old_key_id.clone()).await?;
        Ok(new_key_id.clone())
    }

    pub(super) fn read_from_message<'a>(
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

    pub(super) fn read_end_of_message<'a>(
        &'a self,
        message: &'a Vec<u8>,
        start: usize,
    ) -> Result<&'a [u8]> {
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

    pub(super) fn mix_hash(&self, h: &[u8; 32], data: &[u8]) -> [u8; 32] {
        let mut input = h.to_vec();
        input.extend_from_slice(data.as_ref());
        self.sha256(&input)
    }

    pub(super) async fn identity(&self) -> Result<Identity> {
        Ok(self.identity.clone())
    }

    pub(super) async fn credentials(&self) -> Result<Vec<Credential>> {
        Ok(self.credentials.clone())
    }

    pub(super) async fn sign_static_key(&self, key_id: &KeyId) -> Result<Signature> {
        let public_static_key = self.vault.secret_public_key_get(key_id).await?;
        self.identities_keys()
            .create_signature(&self.identity().await?, public_static_key.data(), None)
            .await
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
        } else if !self.credentials.is_empty() {
            //we cannot validate credentials without a trust context
            return Err(IdentityError::SecureChannelVerificationFailed.into());
        };
        Ok(())
    }

    fn sha256(&self, data: &[u8]) -> [u8; 32] {
        let digest = Sha256::digest(data);
        *array_ref![digest, 0, 32]
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
    pub(super) n: usize,
    pub(super) h: [u8; SHA256_SIZE_USIZE],
    pub(super) ck: KeyId,
    pub(super) prologue: Vec<u8>,
    pub(super) status: T,
}

impl<T> State<T> {
    pub(super) fn new(status: T) -> State<T> {
        State {
            s: "".to_string(),
            e: "".to_string(),
            k: "".to_string(),
            n: 0,
            h: [0u8; SHA256_SIZE_USIZE],
            ck: "".to_string(),
            prologue: vec![],
            status,
        }
    }
}
