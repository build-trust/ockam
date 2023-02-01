use crate::vault::{
    AsymmetricVault, KeyAttributes, KeyPersistence, KeyType, KeyVault, CURVE25519_SECRET_LENGTH_U32,
};

pub async fn ec_diffie_hellman_curve25519(vault: &mut (impl AsymmetricVault + KeyVault)) {
    let attributes = KeyAttributes::new(
        KeyType::X25519,
        KeyPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH_U32,
    );
    let sk_ctx_1 = vault.generate_key(attributes).await.unwrap();
    let sk_ctx_2 = vault.generate_key(attributes).await.unwrap();
    let pk_1 = vault.get_public_key(&sk_ctx_1).await.unwrap();
    let pk_2 = vault.get_public_key(&sk_ctx_2).await.unwrap();

    let res1 = vault.ec_diffie_hellman(&sk_ctx_1, &pk_2).await;
    assert!(res1.is_ok());
    let _ss1 = res1.unwrap();

    let res2 = vault.ec_diffie_hellman(&sk_ctx_2, &pk_1).await;
    assert!(res2.is_ok());
    let _ss2 = res2.unwrap();
    // TODO: Check result against test vector
}
