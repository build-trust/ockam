use ockam_core::hex::decode;
use ockam_core::vault::{
    AsymmetricVault, Buffer, Hasher, PublicKey, Secret, SecretAttributes, SecretKey,
    SecretPersistence, SecretType, SecretVault, SmallBuffer, SymmetricVault,
    CURVE25519_SECRET_LENGTH,
};
use ockam_core::{async_trait, compat::boxed::Box};
use ockam_core::{AsyncTryClone, Result};
use ockam_key_exchange_xx::XXVault;

#[derive(AsyncTryClone)]
struct VaultMock<V: XXVault> {
    count: i8,
    ephemeral_private: Secret,
    vault: V,
}

impl<V: XXVault> VaultMock<V> {
    async fn new(vault: &V, ephemeral_private: &str) -> Self {
        let vault = vault.async_try_clone().await.unwrap();

        let ephemeral_private = vault
            .secret_import(
                &decode(ephemeral_private).unwrap(),
                SecretAttributes::new(
                    SecretType::X25519,
                    SecretPersistence::Ephemeral,
                    CURVE25519_SECRET_LENGTH,
                ),
            )
            .await
            .unwrap();

        Self {
            count: 0,
            ephemeral_private,
            vault,
        }
    }
}

#[async_trait]
impl<V: XXVault> SecretVault for VaultMock<V> {
    async fn secret_generate(&self, _attributes: SecretAttributes) -> Result<Secret> {
        if self.count == 0 {
            Ok(self.ephemeral_private.clone())
        } else {
            unimplemented!()
        }
    }

    async fn secret_import(&self, secret: &[u8], attributes: SecretAttributes) -> Result<Secret> {
        self.vault.secret_import(secret, attributes).await
    }

    async fn secret_export(&self, context: &Secret) -> Result<SecretKey> {
        self.vault.secret_export(context).await
    }

    async fn secret_attributes_get(&self, context: &Secret) -> Result<SecretAttributes> {
        self.vault.secret_attributes_get(context).await
    }

    async fn secret_public_key_get(&self, context: &Secret) -> ockam_core::Result<PublicKey> {
        self.vault.secret_public_key_get(context).await
    }

    async fn secret_destroy(&self, context: Secret) -> ockam_core::Result<()> {
        self.vault.secret_destroy(context).await
    }
}

#[async_trait]
impl<V: XXVault> Hasher for VaultMock<V> {
    async fn sha256(&self, data: &[u8]) -> Result<[u8; 32]> {
        self.vault.sha256(data).await
    }

    async fn hkdf_sha256(
        &self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: SmallBuffer<SecretAttributes>,
    ) -> Result<SmallBuffer<Secret>> {
        self.vault
            .hkdf_sha256(salt, info, ikm, output_attributes)
            .await
    }
}

#[async_trait]
impl<V: XXVault> AsymmetricVault for VaultMock<V> {
    async fn ec_diffie_hellman(
        &self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret> {
        self.vault.ec_diffie_hellman(context, peer_public_key).await
    }
}

#[async_trait]
impl<V: XXVault> SymmetricVault for VaultMock<V> {
    async fn aead_aes_gcm_encrypt(
        &self,
        context: &Secret,
        plaintext: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.vault
            .aead_aes_gcm_encrypt(context, plaintext, nonce, aad)
            .await
    }

    async fn aead_aes_gcm_decrypt(
        &self,
        context: &Secret,
        cipher_text: &[u8],
        nonce: &[u8],
        aad: &[u8],
    ) -> Result<Buffer<u8>> {
        self.vault
            .aead_aes_gcm_decrypt(context, cipher_text, nonce, aad)
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::VaultMock;
    use ockam_core::hex::{decode, encode};
    use ockam_core::vault::{
        KeyPair, SecretAttributes, SecretPersistence, SecretType, SecretVault,
        CURVE25519_SECRET_LENGTH,
    };
    use ockam_core::AsyncTryClone;
    use ockam_key_exchange_xx::noise::{HandshakePattern, HandshakeState};
    use ockam_key_exchange_xx::XXVault;
    use ockam_vault::Vault;

    #[tokio::test]
    async fn prologue() {
        let vault = Vault::create();

        let exp_h = [
            93, 247, 43, 103, 185, 101, 173, 209, 22, 143, 10, 108, 117, 109, 242, 28, 32, 79, 126,
            100, 252, 104, 43, 230, 163, 171, 75, 104, 44, 141, 182, 75,
        ];

        let state = HandshakeState::initialize(
            HandshakePattern::new_xx(),
            true,
            &[],
            None,
            None,
            None,
            None,
            vault.async_try_clone().await.unwrap(),
        )
        .await
        .unwrap();

        let h = state.symmetric_state().get_handshake_hash().unwrap();

        assert_eq!(h, exp_h);

        let ck = vault
            .secret_export(&state.symmetric_state().ck())
            .await
            .unwrap();

        assert_eq!(ck.as_ref(), *b"Noise_XX_25519_AESGCM_SHA256\0\0\0\0");
        assert_eq!(state.symmetric_state().cipher_state().n(), 0);
    }

    const INIT_STATIC: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    const INIT_EPH: &str = "202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f";
    const RESP_STATIC: &str = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
    const RESP_EPH: &str = "4142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f60";

    #[tokio::test]
    async fn handshake_1() {
        const MSG_1_CIPHERTEXT: &str =
            "358072d6365880d1aeea329adf9121383851ed21a28e3b75e965d0d2cd166254";
        const MSG_1_PAYLOAD: &str = "";
        const MSG_2_CIPHERTEXT: &str = "64b101b1d0be5a8704bd078f9895001fc03e8e9f9522f188dd128d9846d484665393019dbd6f438795da206db0886610b26108e424142c2e9b5fd1f7ea70cde8767ce62d7e3c0e9bcefe4ab872c0505b9e824df091b74ffe10a2b32809cab21f";
        const MSG_2_PAYLOAD: &str = "";
        const MSG_3_CIPHERTEXT: &str = "e610eadc4b00c17708bf223f29a66f02342fbedf6c0044736544b9271821ae40e70144cecd9d265dffdc5bb8e051c3f83db32a425e04d8f510c58a43325fbc56";
        const MSG_3_PAYLOAD: &str = "";

        let mut vault = Vault::create();

        mock_handshake(
            &mut vault,
            INIT_STATIC,
            INIT_EPH,
            RESP_STATIC,
            RESP_EPH,
            MSG_1_PAYLOAD,
            MSG_1_CIPHERTEXT,
            MSG_2_PAYLOAD,
            MSG_2_CIPHERTEXT,
            MSG_3_PAYLOAD,
            MSG_3_CIPHERTEXT,
        )
        .await;
    }

    #[tokio::test]
    async fn handshake_2() {
        const MSG_1_CIPHERTEXT: &str =
            "358072d6365880d1aeea329adf9121383851ed21a28e3b75e965d0d2cd166254746573745f6d73675f30";
        const MSG_1_PAYLOAD: &str = "746573745f6d73675f30";
        const MSG_2_PAYLOAD: &str = "746573745f6d73675f31";
        const MSG_2_CIPHERTEXT: &str = "64b101b1d0be5a8704bd078f9895001fc03e8e9f9522f188dd128d9846d484665393019dbd6f438795da206db0886610b26108e424142c2e9b5fd1f7ea70cde8c9f29dcec8d3ab554f4a5330657867fe4917917195c8cf360e08d6dc5f71baf875ec6e3bfc7afda4c9c2";
        const MSG_3_PAYLOAD: &str = "746573745f6d73675f32";
        const MSG_3_CIPHERTEXT: &str = "e610eadc4b00c17708bf223f29a66f02342fbedf6c0044736544b9271821ae40232c55cd96d1350af861f6a04978f7d5e070c07602c6b84d25a331242a71c50ae31dd4c164267fd48bd2";

        let mut vault = Vault::create();

        mock_handshake(
            &mut vault,
            INIT_STATIC,
            INIT_EPH,
            RESP_STATIC,
            RESP_EPH,
            MSG_1_PAYLOAD,
            MSG_1_CIPHERTEXT,
            MSG_2_PAYLOAD,
            MSG_2_CIPHERTEXT,
            MSG_3_PAYLOAD,
            MSG_3_CIPHERTEXT,
        )
        .await;
    }

    async fn mock_handshake<V: XXVault>(
        vault: &mut V,
        init_static: &'static str,
        init_eph: &'static str,
        resp_static: &'static str,
        resp_eph: &'static str,
        msg_1_payload: &'static str,
        msg_1_ciphertext: &'static str,
        msg_2_payload: &'static str,
        msg_2_ciphertext: &'static str,
        msg_3_payload: &'static str,
        msg_3_ciphertext: &'static str,
    ) {
        let mut initiator = mock_prologue(vault, true, init_static, init_eph).await;
        let mut responder = mock_prologue(vault, false, resp_static, resp_eph).await;

        let mut payload = vec![];

        let mut msg1 = vec![];
        let res = initiator
            .write_message(&decode(msg_1_payload).unwrap(), &mut msg1)
            .await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
        assert_eq!(encode(&msg1), msg_1_ciphertext);

        let res = responder.read_message(&msg1, &mut payload).await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
        assert_eq!(encode(&payload), msg_1_payload);
        payload.clear();

        let mut msg2 = vec![];
        let res = responder
            .write_message(&decode(msg_2_payload).unwrap(), &mut msg2)
            .await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
        assert_eq!(encode(&msg2), msg_2_ciphertext);

        let res = initiator.read_message(&msg2, &mut payload).await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
        assert_eq!(encode(&payload), msg_2_payload);
        payload.clear();

        let mut msg3 = vec![];
        let res = initiator
            .write_message(&decode(msg_3_payload).unwrap(), &mut msg3)
            .await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_some());
        assert_eq!(encode(&msg3), msg_3_ciphertext);

        let res = responder.read_message(&msg3, &mut payload).await;
        assert!(res.is_ok());
        assert!(res.unwrap().is_some());
    }

    async fn mock_prologue<V: XXVault>(
        vault: &mut V,
        initiator: bool,
        static_private: &str,
        ephemeral_private: &str,
    ) -> HandshakeState<VaultMock<V>> {
        let vault_mock = VaultMock::new(vault, ephemeral_private).await;

        let attributes = SecretAttributes::new(
            SecretType::X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        );
        // Static x25519 for this handshake, `s`
        let static_secret_handle = vault
            .secret_import(&decode(static_private).unwrap(), attributes)
            .await
            .unwrap();
        let static_public_key = vault
            .secret_public_key_get(&static_secret_handle)
            .await
            .unwrap();

        let handshake_pattern = HandshakePattern::new_xx();

        HandshakeState::initialize(
            handshake_pattern,
            initiator,
            &[],
            Some(KeyPair::new(static_secret_handle, static_public_key)),
            None,
            None,
            None,
            vault_mock,
        )
        .await
        .unwrap()
    }
}
