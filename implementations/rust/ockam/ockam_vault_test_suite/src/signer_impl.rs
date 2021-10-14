use ockam_vault_core::{
    SecretAttributes, SecretPersistence, SecretType, SecretVault, Signer, Verifier,
    CURVE25519_SECRET_LENGTH,
};

pub async fn sign(vault: &mut (impl Signer + Verifier + SecretVault)) {
    let secret = vault
        .secret_generate(SecretAttributes::new(
            SecretType::Curve25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        ))
        .await
        .unwrap();
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
