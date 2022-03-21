use crate::noise::{CipherState, HandshakePattern, HandshakeState};
use crate::{XXCipher, XXError, XXVault};
use ockam_core::compat::{
    string::{String, ToString},
    vec::Vec,
};
use ockam_core::vault::{
    KeyPair, SecretAttributes, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH,
};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_key_exchange_core::{CompletedKeyExchange, KeyExchanger};

/// Represents an XX initiator
#[derive(Debug)]
pub struct KeyExchange<V: XXVault> {
    initiator: bool,
    state: HandshakeState<V>,
    res: Option<(CipherState<V>, CipherState<V>)>,
}

impl<V: XXVault> KeyExchange<V> {
    pub(crate) async fn new(initiator: bool, vault: V) -> Result<Self> {
        let s = vault
            .secret_generate(SecretAttributes::new(
                SecretType::X25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            ))
            .await?;
        let s_pub = vault.secret_public_key_get(&s).await?;
        let s = KeyPair::new(s, s_pub);

        let handshake_pattern = HandshakePattern::new_xx();

        let state = HandshakeState::initialize(
            handshake_pattern,
            initiator,
            &[],
            Some(s),
            None,
            None,
            None,
            vault,
        )
        .await?;

        Ok(Self {
            initiator,
            state,
            res: None,
        })
    }
}

#[async_trait]
impl<V: XXVault> KeyExchanger for KeyExchange<V> {
    type Cipher = XXCipher<V>;

    async fn name(&self) -> Result<String> {
        Ok("NOISE_XX".to_string())
    }

    async fn generate_request(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        if self.is_complete().await? {
            return Err(XXError::InvalidState.into());
        }

        let mut vec = vec![];
        self.res = self.state.write_message(payload, &mut vec).await?;

        Ok(vec)
    }

    async fn handle_response(&mut self, response: &[u8]) -> Result<Vec<u8>> {
        if self.is_complete().await? {
            return Err(XXError::InvalidState.into());
        }

        let mut vec = vec![];
        self.res = self.state.read_message(response, &mut vec).await?;

        Ok(vec)
    }

    async fn is_complete(&self) -> Result<bool> {
        Ok(self.res.is_some())
    }

    async fn finalize(self) -> Result<CompletedKeyExchange<Self::Cipher>> {
        let res = self.res.ok_or(XXError::InvalidState)?;

        let h = self.state.symmetric_state().get_handshake_hash()?;

        let completed = if self.initiator {
            CompletedKeyExchange::new(h, XXCipher::new(res.0), XXCipher::new(res.1))
        } else {
            CompletedKeyExchange::new(h, XXCipher::new(res.1), XXCipher::new(res.0))
        };

        Ok(completed)
    }
}
