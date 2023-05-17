use crate::secure_channel::handshake::error::XXError;
use crate::secure_channel::handshake::handshake_state_machine::{HandshakeResults, Status};
use crate::secure_channel::Role;
use crate::{
    Credential, Credentials, Identities, Identity, IdentityError, SecureChannelTrustInfo,
    TrustContext, TrustPolicy, XXVault,
};
use arrayref::array_ref;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::vault::SecretType::X25519;
use ockam_core::vault::{
    KeyId, PublicKey, Secret, SecretAttributes, SecretKey, SecretPersistence, SecretType,
    Signature, AES256_SECRET_LENGTH_U32, CURVE25519_PUBLIC_LENGTH_USIZE,
    CURVE25519_SECRET_LENGTH_U32,
};
use ockam_core::{Error, Message, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::info;
use SecretType::*;
use Status::*;

/// The number of bytes in a SHA256 digest
pub const SHA256_SIZE_U32: u32 = 32;
/// The number of bytes in a SHA256 digest
pub const SHA256_SIZE_USIZE: usize = 32;
/// The number of bytes in AES-GCM tag
pub const AES_GCM_TAGSIZE_USIZE: usize = 16;

/// Implementation of a Handshake for the noise protocol
/// The first members are used in the implementation of some of the protocol steps, for example to
/// encrypt messages
/// The variables used in the protocol itself: s, e, rs, re,... are handled in `HandshakeState`
pub(super) struct Handshake {
    vault: Arc<dyn XXVault>,
    identities: Arc<Identities>,
    trust_policy: Arc<dyn TrustPolicy>,
    trust_context: Option<TrustContext>,
    credentials_expected: bool,
    pub(super) state: HandshakeState,
}

/// Top-level functions used in the initiator and responder state machines
/// Each function makes mutable copy of the state to modify it in order to make the code more compact
/// and avoid self.state.xxx = ...
impl Handshake {
    /// Initialize the handshake variables
    pub(super) async fn initialize(&mut self) -> Result<()> {
        let mut state = self.state.clone();
        state.h = Self::protocol_name().clone();
        state.ck = self
            .import_ck_secret(Self::protocol_name().to_vec())
            .await?;
        state.mix_hash(state.prologue.clone().as_slice());

        Ok(self.state = state)
    }

    /// Encode the first message, sent from the initiator to the responder
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

    /// Decode the first message to get the ephemeral public key sent by the initiator
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

    /// Encode the second message from the responder to the initiator
    /// That message contains: the responder ephemeral public key + a Diffie-Hellman key +
    ///   an encrypted payload containing the responder identity / signature / credentials
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

    /// Decode the second message sent by the responder
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

    /// Encode the third message from the initiator to the responder
    /// That message contains: the initiator static public key (encrypted) + a Diffie-Hellman key +
    ///   an encrypted payload containing the initiator identity / signature / credentials
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

    /// Decode the third message sent by the initiator
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

    /// Verify the identity sent by the other party: the signature and the credentials must be valid
    pub(super) async fn verify_identity(&self, peer: IdentityAndCredentials) -> Result<Identity> {
        let identity = self.decode_identity(peer.identity).await?;
        self.verify_signature(&identity, &peer.signature, &self.state.rs)
            .await?;
        self.verify_credentials(&identity, peer.credentials).await?;
        Ok(identity)
    }

    /// Set the final state of the state machine by creating the encryption / decryption keys
    /// and return the other party identity
    pub(super) async fn set_final_state(
        &mut self,
        their_identity: Identity,
        role: Role,
    ) -> Result<()> {
        let mut state = self.state.clone();
        // k1, k2 = HKDF(ck, zerolen, 2)
        let (k1, k2) = self.hkdf(&state.ck, &state.k, None).await?;
        let (encryption_key, decryption_key) = if role.is_initiator() {
            (k2, k1)
        } else {
            (k1, k2)
        };
        state.status = Ready(HandshakeResults {
            their_identity,
            encryption_key,
            decryption_key,
        });

        Ok(self.state = state)
    }

    /// Return the final results of the handshake if we reached the final state
    pub(super) fn get_handshake_results(&self) -> Option<HandshakeResults> {
        match self.state.status.clone() {
            Ready(results) => Some(results),
            _ => None,
        }
    }
}

impl Handshake {
    /// Create a new handshake initialized with
    ///   - a static key
    ///   - an ephemeral key
    ///   - a payload containing the identity, a signature of the static key and the credential of
    ///     the current party
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
            identity: identity.export()?,
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
            state: HandshakeState::new(s, e, identity_payload),
        })
    }

    /// Import the ck secret
    async fn import_ck_secret(&self, content: Vec<u8>) -> Result<KeyId> {
        self.vault
            .secret_import(Secret::Key(SecretKey::new(content)), Self::ck_attributes())
            .await
    }

    /// Return the public key corresponding to a given key id
    async fn get_public_key(&self, key_id: &KeyId) -> Result<PublicKey> {
        self.vault.secret_public_key_get(key_id).await
    }

    /// Compute a Diffie-Hellman key between a given key id and the other party public key
    async fn dh(&self, key_id: &KeyId, public_key: &PublicKey) -> Result<KeyId> {
        self.vault.ec_diffie_hellman(key_id, public_key).await
    }

    /// Compute two derived ck, and k keys based on existing ck and k keys + an optional
    /// Diffie-Hellman key
    async fn hkdf(&mut self, ck: &KeyId, k: &KeyId, dh: Option<&KeyId>) -> Result<(KeyId, KeyId)> {
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

    /// Decrypt a ciphertext 'c' using the key 'k' and the additional data 'h'
    async fn decrypt(&mut self, k: &KeyId, h: &[u8], c: &[u8]) -> Result<Vec<u8>> {
        self.state.n += 1;
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.state.n.to_be_bytes());
        self.vault
            .aead_aes_gcm_decrypt(k, c, nonce.as_ref(), h)
            .await
            .map(|b| b.to_vec())
    }

    /// Encrypt a plaintext 'c' using the key 'k' and the additional data 'h'
    async fn encrypt(&mut self, k: &KeyId, h: &[u8], c: &[u8]) -> Result<Vec<u8>> {
        self.state.n += 1;
        let mut nonce = [0u8; 12];
        nonce[10..].copy_from_slice(&self.state.n.to_be_bytes());
        self.vault
            .aead_aes_gcm_encrypt(k, c, nonce.as_ref(), h)
            .await
            .map(|b| b.to_vec())
    }

    /// Decode an Identity that was encoded with a BARE encoding
    async fn decode_identity(&self, encoded: Vec<u8>) -> Result<Identity> {
        self.identities
            .identities_creation()
            .import_identity(&encoded.as_slice())
            .await
    }

    /// Verify that the signature was signed with the public key associated to the other party identity
    async fn verify_signature(
        &self,
        their_identity: &Identity,
        their_signature: &Signature,
        their_public_key: &PublicKey,
    ) -> Result<()> {
        //verify the signature of the static key used during noise exchanges
        //actually matches the signature of the identity
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
                    .identities
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
}

/// Static functions
impl Handshake {
    /// Protocol name, used as a secret during the handshake initialization, padded to 32 bytes
    fn protocol_name() -> &'static [u8; 32] {
        b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0"
    }

    /// Generate a static key for the key exchange
    /// At the moment that key is actually an ephemeral key (not persisted if the current process stops)
    async fn generate_static_key(vault: Arc<dyn XXVault>) -> Result<KeyId> {
        let attributes = SecretAttributes::new(
            X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        );
        vault.secret_generate(attributes).await
    }

    /// Generate an ephemeral key for the key exchange
    async fn generate_ephemeral_key(vault: Arc<dyn XXVault>) -> Result<KeyId> {
        let attributes = SecretAttributes::new(
            X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        );
        vault.secret_generate(attributes).await
    }

    /// Sign the static key used in the key exchange with the identity private key
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

    /// Deserialize a payload as D from a bare encoding
    fn deserialize<D: for<'a> Deserialize<'a>>(payload: Vec<u8>) -> Result<D> {
        serde_bare::from_slice(payload.as_slice())
            .map_err(|error| Error::new(Origin::Channel, Kind::Invalid, error))
    }

    /// Secret attributes for the ck key
    fn ck_attributes() -> SecretAttributes {
        SecretAttributes::new(Buffer, SecretPersistence::Ephemeral, SHA256_SIZE_U32)
    }

    /// Secret attributes for the k key
    fn k_attributes() -> SecretAttributes {
        SecretAttributes::new(Aes, SecretPersistence::Ephemeral, AES256_SECRET_LENGTH_U32)
    }

    /// Read the message 1 payload which is present after the public key
    fn read_message1_payload(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_end(message, Self::key_size())
    }

    /// Read the message 2 encrypted key, which is present after the public key
    fn read_message2_encrypted_key(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_middle(message, Self::key_size(), Self::encrypted_key_size())
    }

    /// Read the message 2 encrypted payload, which is present after the encrypted key
    fn read_message2_payload(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_end(message, Self::key_size() + Self::encrypted_key_size())
    }

    /// Read the message 3 encrypted key at the beginning of the message
    fn read_message3_encrypted_key(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_start(message, Self::encrypted_key_size())
    }

    /// Read the message 3 payload which is present after the encrypted key
    fn read_message3_payload(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_end(message, Self::encrypted_key_size())
    }

    /// Read the first 'length' bytes of the message
    fn read_start(message: &Vec<u8>, length: usize) -> Result<&[u8]> {
        if message.len() < length {
            return Err(XXError::MessageLenMismatch.into());
        }
        Ok(&message[0..length])
    }

    /// Read the bytes of the message after the first 'drop_length' bytes
    fn read_end(message: &Vec<u8>, drop_length: usize) -> Result<&[u8]> {
        if message.len() < drop_length {
            return Err(XXError::MessageLenMismatch.into());
        }
        Ok(&message[drop_length..])
    }

    /// Read 'length' bytes of the message after the first 'drop_length' bytes
    fn read_middle(message: &Vec<u8>, drop_length: usize, length: usize) -> Result<&[u8]> {
        if message.len() < drop_length + length {
            return Err(XXError::MessageLenMismatch.into());
        }
        Ok(&message[drop_length..(drop_length + length)])
    }

    /// Read the bytes of a key at the beginning of a message
    fn read_key(message: &Vec<u8>) -> Result<&[u8]> {
        Self::read_start(message, Self::key_size())
    }

    /// Size of a public key
    fn key_size() -> usize {
        CURVE25519_PUBLIC_LENGTH_USIZE
    }

    /// Size of an encrypted key
    fn encrypted_key_size() -> usize {
        Self::key_size() + AES_GCM_TAGSIZE_USIZE
    }
}

/// The `HandshakeState` contains all the variables necessary to follow the Noise protocol
#[derive(Debug, Clone)]
pub(super) struct HandshakeState {
    s: KeyId,
    e: KeyId,
    k: KeyId,
    re: PublicKey,
    rs: PublicKey,
    n: usize,
    h: [u8; SHA256_SIZE_USIZE],
    ck: KeyId,
    prologue: Vec<u8>,
    message1_payload: Vec<u8>,
    identity_payload: Vec<u8>,
    pub(super) status: Status,
}

impl HandshakeState {
    /// Create a new HandshakeState with:
    ///   - a static key
    ///   - an ephemeral key
    ///   - an encoded identity payload (identity + signature + credentials)
    pub(super) fn new(s: KeyId, e: KeyId, identity_payload: Vec<u8>) -> HandshakeState {
        HandshakeState {
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

    /// h = SHA256(h || data)
    fn mix_hash(&mut self, data: &[u8]) {
        let mut input = self.h.to_vec();
        input.extend(data);
        let digest = Sha256::digest(data);
        self.h = *array_ref![digest, 0, 32];
    }
}

/// This internal structure is used as a payload in the XX protocol
#[derive(Debug, Clone, Serialize, Deserialize, Message)]
pub(super) struct IdentityAndCredentials {
    // exported identity
    pub(super) identity: Vec<u8>,
    // The signature guarantees that the other end has access to the private key of the identity
    // The signature refers to the static key of the noise ('x') and is made with the static
    // key of the identity
    pub(super) signature: Signature,
    pub(super) credentials: Vec<Credential>,
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use ockam_core::Result;
//     use ockam_core::{KeyExchanger, NewKeyExchanger};
//     use ockam_node::Context;
//     use ockam_vault::Vault;
//
//     #[allow(non_snake_case)]
//     #[ockam_macros::test]
//     async fn full_flow__correct_credential__keys_should_match(ctx: &mut Context) -> Result<()> {
//         let vault = Vault::create();
//
//         let key_exchanger = XXNewKeyExchanger::new(vault.clone());
//
//         let mut initiator = key_exchanger.initiator(None).await.unwrap();
//         let mut responder = key_exchanger.responder(None).await.unwrap();
//
//         loop {
//             if !initiator.is_complete().await.unwrap() {
//                 let m = initiator.generate_request(&[]).await.unwrap();
//                 let _ = responder.handle_response(&m).await.unwrap();
//             }
//
//             if !responder.is_complete().await.unwrap() {
//                 let m = responder.generate_request(&[]).await.unwrap();
//                 let _ = initiator.handle_response(&m).await.unwrap();
//             }
//
//             if initiator.is_complete().await.unwrap() && responder.is_complete().await.unwrap() {
//                 break;
//             }
//         }
//
//         let initiator = initiator.finalize().await.unwrap();
//         let responder = responder.finalize().await.unwrap();
//
//         assert_eq!(initiator.h(), responder.h());
//
//         let s1 = vault.secret_export(initiator.encrypt_key()).await.unwrap();
//         let s2 = vault.secret_export(responder.decrypt_key()).await.unwrap();
//
//         assert_eq!(s1, s2);
//
//         let s1 = vault.secret_export(initiator.decrypt_key()).await.unwrap();
//         let s2 = vault.secret_export(responder.encrypt_key()).await.unwrap();
//
//         assert_eq!(s1, s2);
//
//         ctx.stop().await
//     }
// }
