use crate::noise::{
    CipherState, HandshakePattern, PatternToken, PreMessagePattern, SymmetricState,
};
use crate::{XXError, XXVault};
use ockam_core::compat::collections::VecDeque;
use ockam_core::compat::vec::Vec;
use ockam_core::vault::{
    KeyPair, PublicKey, SecretAttributes, SecretPersistence, SecretType, CURVE25519_PUBLIC_LENGTH,
    CURVE25519_SECRET_LENGTH,
};
use ockam_core::Result;

/// A HandshakeState object contains a SymmetricState plus DH variables (s, e, rs, re)
/// and a variable representing the handshake pattern.
/// During the handshake phase each party has a single HandshakeState,
/// which can be deleted once the handshake is finished.
#[derive(Debug)]
pub struct HandshakeState<V: XXVault> {
    symmetric_state: SymmetricState<V>,
    s: Option<KeyPair>,
    e: Option<KeyPair>,
    rs: Option<PublicKey>,
    re: Option<PublicKey>,
    initiator: bool,
    message_patterns: VecDeque<Vec<PatternToken>>,
    vault: V,
}

impl<V: XXVault> HandshakeState<V> {
    /// SymmetricState
    pub fn symmetric_state(&self) -> &SymmetricState<V> {
        &self.symmetric_state
    }
}

impl<V: XXVault> HandshakeState<V> {
    async fn handle_pre_msg_pattern(
        pattern: PreMessagePattern,
        s: Option<&PublicKey>,
        e: Option<&PublicKey>,
        symmetric_state: &mut SymmetricState<V>,
    ) -> Result<()> {
        match pattern {
            PreMessagePattern::e => {
                let e = e.ok_or(XXError::InvalidPreMsgPatternSetup)?;
                symmetric_state.mix_hash(e.as_ref()).await?;
            }
            PreMessagePattern::s => {
                let s = s.ok_or(XXError::InvalidPreMsgPatternSetup)?;
                symmetric_state.mix_hash(s.as_ref()).await?;
            }
            PreMessagePattern::es => {
                let e = e.ok_or(XXError::InvalidPreMsgPatternSetup)?;
                symmetric_state.mix_hash(e.as_ref()).await?;
                let s = s.ok_or(XXError::InvalidPreMsgPatternSetup)?;
                symmetric_state.mix_hash(s.as_ref()).await?;
            }
            PreMessagePattern::Empty => {}
        }

        Ok(())
    }

    async fn mix_key(
        secret: &Option<KeyPair>,
        public: &Option<PublicKey>,
        symmetric_state: &mut SymmetricState<V>,
        vault: &V,
    ) -> Result<()> {
        let secret = secret
            .as_ref()
            .ok_or(XXError::InvalidHandshakePatternSetup)?
            .secret();
        let public = public
            .as_ref()
            .ok_or(XXError::InvalidHandshakePatternSetup)?;

        let dh = vault.ec_diffie_hellman(secret, public).await?;
        symmetric_state.mix_key(&dh).await
    }
}

impl<V: XXVault> HandshakeState<V> {
    /// Takes a valid handshake_pattern (see Section 7)
    /// and an initiator boolean specifying this party's role as either initiator or responder.
    ///
    /// Takes a prologue byte sequence which may be zero-length, or which may contain context
    /// information that both parties want to confirm is identical (see Section 6).
    ///
    /// Takes a set of DH key pairs (s, e) and public keys (rs, re) for initializing local variables,
    /// any of which may be empty. Public keys are only passed in if the handshake_pattern uses
    /// pre-messages (see Section 7). The ephemeral values (e, re) are typically left empty,
    /// since they are created and exchanged during the handshake;
    /// but there are exceptions (see Section 10).
    ///
    /// Performs the following steps:
    ///
    /// Derives a protocol_name byte sequence by combining the names for the handshake pattern
    /// and crypto functions, as specified in Section 8.
    ///
    /// Calls InitializeSymmetric(protocol_name).
    ///
    /// Calls MixHash(prologue).
    ///
    /// Sets the initiator, s, e, rs, and re variables to the corresponding arguments.
    ///
    /// Calls MixHash() once for each public key listed in the pre-messages from handshake_pattern,
    /// with the specified public key as input (see Section 7 for an explanation of pre-messages).
    /// If both initiator and responder have pre-messages, the initiator's public keys are hashed
    /// first. If multiple public keys are listed in either party's pre-message,
    /// the public keys are hashed in the order that they are listed.
    ///
    /// Sets message_patterns to the message patterns from handshake_pattern.
    #[allow(clippy::too_many_arguments)]
    pub async fn initialize(
        handshake_pattern: HandshakePattern,
        initiator: bool,
        prologue: &[u8],
        s: Option<KeyPair>,
        e: Option<KeyPair>,
        rs: Option<PublicKey>,
        re: Option<PublicKey>,
        vault: V,
    ) -> Result<Self> {
        // TODO: This should change if we want to support something else
        let protocol_name = "Noise_XX_25519_AESGCM_SHA256";
        let mut symmetric_state =
            SymmetricState::initialize_symmetric(protocol_name, vault.async_try_clone().await?)
                .await?;
        symmetric_state.mix_hash(prologue).await?;

        let initiator_s_pub;
        let initiator_e_pub;
        let responder_s_pub;
        let responder_e_pub;

        if initiator {
            initiator_s_pub = s.clone().map(|v| v.public().clone());
            initiator_e_pub = e.clone().map(|v| v.public().clone());
            responder_s_pub = rs.clone();
            responder_e_pub = re.clone();
        } else {
            responder_s_pub = s.clone().map(|v| v.public().clone());
            responder_e_pub = e.clone().map(|v| v.public().clone());
            initiator_s_pub = rs.clone();
            initiator_e_pub = re.clone();
        }

        Self::handle_pre_msg_pattern(
            handshake_pattern.initiator_pre_msg,
            initiator_s_pub.as_ref(),
            initiator_e_pub.as_ref(),
            &mut symmetric_state,
        )
        .await?;
        Self::handle_pre_msg_pattern(
            handshake_pattern.responder_pre_msg,
            responder_s_pub.as_ref(),
            responder_e_pub.as_ref(),
            &mut symmetric_state,
        )
        .await?;

        Ok(Self {
            symmetric_state,
            s,
            e,
            rs,
            re,
            initiator,
            message_patterns: handshake_pattern.message_patterns,
            vault,
        })
    }

    /// Takes a payload byte sequence which may be zero-length,
    /// and a message_buffer to write the output into.
    ///
    /// Performs the following steps, aborting if any EncryptAndHash() call returns an error:
    ///
    /// Fetches and deletes the next message pattern from message_patterns,
    /// then sequentially processes each token from the message pattern:
    ///
    ///    - For "e": Sets e (which must be empty) to GENERATE_KEYPAIR().
    ///     Appends e.public_key to the buffer. Calls MixHash(e.public_key).
    ///
    ///    - For "s": Appends EncryptAndHash(s.public_key) to the buffer.
    ///
    ///    - For "ee": Calls MixKey(DH(e, re)).
    ///
    ///    - For "es": Calls MixKey(DH(e, rs)) if initiator, MixKey(DH(s, re)) if responder.
    ///
    ///    - For "se": Calls MixKey(DH(s, re)) if initiator, MixKey(DH(e, rs)) if responder.
    ///
    ///    - For "ss": Calls MixKey(DH(s, rs)).
    ///
    /// Appends EncryptAndHash(payload) to the buffer.
    ///
    /// If there are no more message patterns returns two new CipherState objects by calling Split().
    pub async fn write_message(
        &mut self,
        payload: &[u8],
        buffer: &mut Vec<u8>,
    ) -> Result<Option<(CipherState<V>, CipherState<V>)>> {
        let next_pattern = self
            .message_patterns
            .pop_front()
            .ok_or(XXError::NoPatternsLeft)?;

        let mut res = vec![];

        for token in next_pattern {
            match token {
                PatternToken::e => {
                    if self.e.is_some() {
                        return Err(XXError::InvalidHandshakePatternSetup.into());
                    }

                    let e = self
                        .vault
                        .secret_generate(SecretAttributes::new(
                            SecretType::X25519,
                            SecretPersistence::Ephemeral,
                            CURVE25519_SECRET_LENGTH,
                        ))
                        .await?;
                    let e_pub = self.vault.secret_public_key_get(&e).await?;

                    res.extend_from_slice(e_pub.as_ref());
                    self.symmetric_state.mix_hash(e_pub.as_ref()).await?;

                    self.e = Some(KeyPair::new(e, e_pub));
                }
                PatternToken::s => {
                    let s = self
                        .s
                        .as_ref()
                        .ok_or(XXError::InvalidHandshakePatternSetup)?;

                    let mut s = self
                        .symmetric_state
                        .encrypt_and_hash(s.public().as_ref())
                        .await?;
                    res.append(&mut s);
                }
                PatternToken::ee => {
                    Self::mix_key(&self.e, &self.re, &mut self.symmetric_state, &self.vault)
                        .await?;
                }
                PatternToken::es => {
                    if self.initiator {
                        Self::mix_key(&self.e, &self.rs, &mut self.symmetric_state, &self.vault)
                            .await?;
                    } else {
                        Self::mix_key(&self.s, &self.re, &mut self.symmetric_state, &self.vault)
                            .await?;
                    }
                }
                PatternToken::se => {
                    if self.initiator {
                        Self::mix_key(&self.s, &self.re, &mut self.symmetric_state, &self.vault)
                            .await?;
                    } else {
                        Self::mix_key(&self.e, &self.rs, &mut self.symmetric_state, &self.vault)
                            .await?;
                    }
                }
                PatternToken::ss => {
                    Self::mix_key(&self.s, &self.rs, &mut self.symmetric_state, &self.vault)
                        .await?;
                }
            }
        }

        let mut s = self.symmetric_state.encrypt_and_hash(payload).await?;
        res.append(&mut s);

        buffer.append(&mut res);

        if self.message_patterns.is_empty() {
            let cipher = self.symmetric_state.split().await?;
            Ok(Some((cipher.0, cipher.1)))
        } else {
            Ok(None)
        }
    }

    /// Takes a byte sequence containing a Noise handshake message,
    /// and a payload_buffer to write the message's plaintext payload into.
    ///
    /// Performs the following steps, aborting if any DecryptAndHash() call returns an error:
    ///
    /// Fetches and deletes the next message pattern from message_patterns,
    /// then sequentially processes each token from the message pattern:
    ///
    ///    - For "e": Sets re (which must be empty) to the next DHLEN bytes from the message.
    ///     Calls MixHash(re.public_key).
    ///
    ///    - For "s": Sets temp to the next DHLEN + 16 bytes of the message if HasKey() == True,
    ///     or to the next DHLEN bytes otherwise.
    ///     Sets rs (which must be empty) to DecryptAndHash(temp).
    ///
    ///    - For "ee": Calls MixKey(DH(e, re)).
    ///
    ///    - For "es": Calls MixKey(DH(e, rs)) if initiator, MixKey(DH(s, re)) if responder.
    ///
    ///    - For "se": Calls MixKey(DH(s, re)) if initiator, MixKey(DH(e, rs)) if responder.
    ///
    ///    - For "ss": Calls MixKey(DH(s, rs)).
    ///
    /// Calls DecryptAndHash() on the remaining bytes of the message
    /// and stores the output into payload_buffer.
    ///
    /// If there are no more message patterns returns two new CipherState objects by calling Split().
    pub async fn read_message(
        &mut self,
        message: &[u8],
        payload_buffer: &mut Vec<u8>,
    ) -> Result<Option<(CipherState<V>, CipherState<V>)>> {
        let next_pattern = self
            .message_patterns
            .pop_front()
            .ok_or(XXError::NoPatternsLeft)?;

        let mut index = 0usize;
        let mut res = vec![];

        for token in next_pattern {
            match token {
                PatternToken::e => {
                    if self.re.is_some() {
                        return Err(XXError::InvalidHandshakePatternSetup.into());
                    }

                    let re = &message[index..CURVE25519_PUBLIC_LENGTH];
                    index += CURVE25519_PUBLIC_LENGTH;
                    let re = PublicKey::new(re.to_vec(), SecretType::X25519);
                    self.symmetric_state.mix_hash(re.as_ref()).await?;

                    self.re = Some(re);
                }
                PatternToken::s => {
                    let temp;
                    if self.symmetric_state.has_key() {
                        temp = &message[index..index + CURVE25519_PUBLIC_LENGTH + 16];
                        index += CURVE25519_PUBLIC_LENGTH + 16;
                    } else {
                        temp = &message[index..index + CURVE25519_PUBLIC_LENGTH];
                        index += CURVE25519_PUBLIC_LENGTH;
                    };

                    if self.rs.is_some() {
                        return Err(XXError::InvalidHandshakePatternSetup.into());
                    }

                    let rs = self.symmetric_state.decrypt_and_hash(temp).await?;
                    let rs = PublicKey::new(rs.to_vec(), SecretType::X25519);
                    self.rs = Some(rs);
                }
                PatternToken::ee => {
                    Self::mix_key(&self.e, &self.re, &mut self.symmetric_state, &self.vault)
                        .await?;
                }
                PatternToken::es => {
                    if self.initiator {
                        Self::mix_key(&self.e, &self.rs, &mut self.symmetric_state, &self.vault)
                            .await?;
                    } else {
                        Self::mix_key(&self.s, &self.re, &mut self.symmetric_state, &self.vault)
                            .await?;
                    }
                }
                PatternToken::se => {
                    if self.initiator {
                        Self::mix_key(&self.s, &self.re, &mut self.symmetric_state, &self.vault)
                            .await?;
                    } else {
                        Self::mix_key(&self.e, &self.rs, &mut self.symmetric_state, &self.vault)
                            .await?;
                    }
                }
                PatternToken::ss => {
                    Self::mix_key(&self.s, &self.rs, &mut self.symmetric_state, &self.vault)
                        .await?;
                }
            }
        }

        let mut s = self
            .symmetric_state
            .decrypt_and_hash(&message[index..])
            .await?;
        res.append(&mut s);

        payload_buffer.append(&mut res);

        if self.message_patterns.is_empty() {
            let cipher = self.symmetric_state.split().await?;
            Ok(Some((cipher.0, cipher.1)))
        } else {
            Ok(None)
        }
    }
}
