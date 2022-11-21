use crate::vault::{
    Secret, SecretAttributes, SecretKey, SecretPersistence, SecretType, SecretVault,
    CURVE25519_PUBLIC_LENGTH_USIZE, CURVE25519_SECRET_LENGTH_U32,
};
use hex::{decode, encode};

pub async fn new_public_keys(vault: &mut impl SecretVault) {
    let attributes = SecretAttributes::new(
        SecretType::Ed25519,
        SecretPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH_U32,
    );

    let res = vault.secret_generate(attributes).await;
    assert!(res.is_ok());
    let ed_ctx_1 = res.unwrap();

    let res = vault.secret_public_key_get(&ed_ctx_1).await;
    assert!(res.is_ok());
    let pk_1 = res.unwrap();
    assert_eq!(pk_1.data().len(), CURVE25519_PUBLIC_LENGTH_USIZE);

    let attributes = SecretAttributes::new(
        SecretType::X25519,
        SecretPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH_U32,
    );
    let res = vault.secret_generate(attributes).await;
    assert!(res.is_ok());
    let x25519_ctx_1 = res.unwrap();
    let res = vault.secret_public_key_get(&x25519_ctx_1).await;
    assert!(res.is_ok());
    let pk_1 = res.unwrap();
    assert_eq!(pk_1.data().len(), CURVE25519_PUBLIC_LENGTH_USIZE);
}

pub async fn new_secret_keys(vault: &mut impl SecretVault) {
    let types = [(SecretType::X25519, 32), (SecretType::Buffer, 24)];
    for (t, s) in &types {
        let attributes = SecretAttributes::new(*t, SecretPersistence::Ephemeral, *s);
        let res = vault.secret_generate(attributes).await;
        assert!(res.is_ok());
        let sk_ctx = res.unwrap();
        let sk = vault.secret_export(&sk_ctx).await.unwrap();
        assert_eq!(sk.cast_as_key().as_ref().len() as u32, *s);
        vault.secret_destroy(sk_ctx).await.unwrap();
    }
}

pub async fn secret_import_export(vault: &mut impl SecretVault) {
    let attributes = SecretAttributes::new(
        SecretType::X25519,
        SecretPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH_U32,
    );

    let secret_str = "98d589b0dce92c9e2442b3093718138940bff71323f20b9d158218b89c3cec6e";

    let secret = vault
        .secret_import(
            Secret::Key(SecretKey::new(decode(secret_str).unwrap())),
            attributes,
        )
        .await
        .unwrap();

    assert_eq!(
        encode(
            vault
                .secret_export(&secret)
                .await
                .unwrap()
                .cast_as_key()
                .as_ref()
        ),
        secret_str
    );

    let attributes = SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, 24u32);
    let secret_str = "5f791cc52297f62c7b8829b15f828acbdb3c613371d21aa1";
    let secret = vault
        .secret_import(
            Secret::Key(SecretKey::new(decode(secret_str).unwrap())),
            attributes,
        )
        .await
        .unwrap();

    assert_eq!(
        encode(
            vault
                .secret_export(&secret)
                .await
                .unwrap()
                .cast_as_key()
                .as_ref()
        ),
        secret_str
    );
}

pub async fn secret_attributes_get(vault: &mut impl SecretVault) {
    let attributes = SecretAttributes::new(
        SecretType::X25519,
        SecretPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH_U32,
    );

    let secret = vault.secret_generate(attributes).await.unwrap();
    assert_eq!(
        vault.secret_attributes_get(&secret).await.unwrap(),
        attributes
    );

    let attributes = SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, 24u32);

    let secret = vault.secret_generate(attributes).await.unwrap();
    assert_eq!(
        vault.secret_attributes_get(&secret).await.unwrap(),
        attributes
    );
}
