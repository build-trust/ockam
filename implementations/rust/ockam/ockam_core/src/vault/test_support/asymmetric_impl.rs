use crate::vault::{
    AsymmetricVault, SecretAttributes, SecretPersistence, SecretType, SecretVault,
    CURVE25519_SECRET_LENGTH_U32,
};

pub async fn ec_diffie_hellman_curve25519(vault: &mut (impl AsymmetricVault + SecretVault)) {
    let attributes = SecretAttributes::new(
        SecretType::X25519,
        SecretPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH_U32,
    );
    let sk_ctx_1 = vault.secret_generate(attributes).await.unwrap();
    let sk_ctx_2 = vault.secret_generate(attributes).await.unwrap();
    let pk_1 = vault.secret_public_key_get(&sk_ctx_1).await.unwrap();
    let pk_2 = vault.secret_public_key_get(&sk_ctx_2).await.unwrap();

    let res1 = vault.ec_diffie_hellman(&sk_ctx_1, &pk_2).await;
    assert!(res1.is_ok());
    let _ss1 = res1.unwrap();

    let res2 = vault.ec_diffie_hellman(&sk_ctx_2, &pk_1).await;
    assert!(res2.is_ok());
    let _ss2 = res2.unwrap();
    // TODO: Check result against test vector
}
