use crate::vault::{
    Key, KeyAttributes, KeyPersistence, KeyType, KeyVault, PrivateKey,
    CURVE25519_PUBLIC_LENGTH_USIZE, CURVE25519_SECRET_LENGTH_U32,
};
use hex::{decode, encode};

pub async fn new_public_keys(vault: &mut impl KeyVault) {
    let attributes = KeyAttributes::new(
        KeyType::Ed25519,
        KeyPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH_U32,
    );

    let res = vault.generate_key(attributes).await;
    assert!(res.is_ok());
    let ed_ctx_1 = res.unwrap();

    let res = vault.get_public_key(&ed_ctx_1).await;
    assert!(res.is_ok());
    let pk_1 = res.unwrap();
    assert_eq!(pk_1.data().len(), CURVE25519_PUBLIC_LENGTH_USIZE);

    let attributes = KeyAttributes::new(
        KeyType::X25519,
        KeyPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH_U32,
    );
    let res = vault.generate_key(attributes).await;
    assert!(res.is_ok());
    let x25519_ctx_1 = res.unwrap();
    let res = vault.get_public_key(&x25519_ctx_1).await;
    assert!(res.is_ok());
    let pk_1 = res.unwrap();
    assert_eq!(pk_1.data().len(), CURVE25519_PUBLIC_LENGTH_USIZE);
}

pub async fn new_secret_keys(vault: &mut impl KeyVault) {
    let types = [(KeyType::X25519, 32), (KeyType::Buffer, 24)];
    for (t, s) in &types {
        let attributes = KeyAttributes::new(*t, KeyPersistence::Ephemeral, *s);
        let res = vault.generate_key(attributes).await;
        assert!(res.is_ok());
        let sk_ctx = res.unwrap();
        let sk = vault.export_key(&sk_ctx).await.unwrap();
        assert_eq!(sk.cast_as_key().as_ref().len() as u32, *s);
        vault.destroy_key(sk_ctx).await.unwrap();
    }
}

pub async fn secret_import_export(vault: &mut impl KeyVault) {
    let attributes = KeyAttributes::new(
        KeyType::X25519,
        KeyPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH_U32,
    );

    let secret_str = "98d589b0dce92c9e2442b3093718138940bff71323f20b9d158218b89c3cec6e";

    let secret = vault
        .import_key(
            Key::Key(PrivateKey::new(decode(secret_str).unwrap())),
            attributes,
        )
        .await
        .unwrap();

    assert_eq!(
        encode(
            vault
                .export_key(&secret)
                .await
                .unwrap()
                .cast_as_key()
                .as_ref()
        ),
        secret_str
    );

    let attributes = KeyAttributes::new(KeyType::Buffer, KeyPersistence::Ephemeral, 24u32);
    let secret_str = "5f791cc52297f62c7b8829b15f828acbdb3c613371d21aa1";
    let secret = vault
        .import_key(
            Key::Key(PrivateKey::new(decode(secret_str).unwrap())),
            attributes,
        )
        .await
        .unwrap();

    assert_eq!(
        encode(
            vault
                .export_key(&secret)
                .await
                .unwrap()
                .cast_as_key()
                .as_ref()
        ),
        secret_str
    );
}

pub async fn secret_attributes_get(vault: &mut impl KeyVault) {
    let attributes = KeyAttributes::new(
        KeyType::X25519,
        KeyPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH_U32,
    );

    let secret = vault.generate_key(attributes).await.unwrap();
    assert_eq!(vault.get_key_attributes(&secret).await.unwrap(), attributes);

    let attributes = KeyAttributes::new(KeyType::Buffer, KeyPersistence::Ephemeral, 24u32);

    let secret = vault.generate_key(attributes).await.unwrap();
    assert_eq!(vault.get_key_attributes(&secret).await.unwrap(), attributes);
}
