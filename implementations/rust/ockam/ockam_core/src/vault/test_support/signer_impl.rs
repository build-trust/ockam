use crate::vault::{
    SecretAttributes, SecretPersistence, SecretType, SecretVault, Signer, Verifier,
    CURVE25519_SECRET_LENGTH_U32,
};

pub async fn sign(vault: &mut (impl Signer + Verifier + SecretVault)) {
    for attributes in [
        SecretAttributes::new(
            SecretType::X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        ),
        SecretAttributes::new(
            SecretType::Ed25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH_U32,
        ),
    ] {
        let secret = vault.secret_generate(attributes).await.unwrap();
        let res = vault.sign(&secret, b"hello world!").await;
        assert!(res.is_ok());
        let pubkey = vault.secret_public_key_get(&secret).await.unwrap();
        let signature = res.unwrap();
        let res = vault
            .verify(&signature, &pubkey, b"hello world!")
            .await
            .unwrap();
        assert!(res);
    }
}
