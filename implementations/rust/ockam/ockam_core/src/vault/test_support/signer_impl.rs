use crate::vault::{
    KeyAttributes, KeyPersistence, KeyType, KeyVault, Signer, Verifier,
    CURVE25519_SECRET_LENGTH_U32,
};

pub async fn sign(vault: &mut (impl Signer + Verifier + KeyVault)) {
    for attributes in [
        KeyAttributes::new(
            KeyType::X25519,
            KeyPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        ),
        KeyAttributes::new(
            KeyType::Ed25519,
            KeyPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        ),
    ] {
        let secret = vault.generate_key(attributes).await.unwrap();
        let res = vault.sign(&secret, b"hello world!").await;
        assert!(res.is_ok());
        let pubkey = vault.get_public_key(&secret).await.unwrap();
        let signature = res.unwrap();
        let res = vault
            .verify(&signature, &pubkey, b"hello world!")
            .await
            .unwrap();
        assert!(res);
    }
}
