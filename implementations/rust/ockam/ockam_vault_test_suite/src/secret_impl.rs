use ockam_core::hex::{decode, encode};
use ockam_vault_core::{
    SecretAttributes, SecretPersistence, SecretType, SecretVault, CURVE25519_PUBLIC_LENGTH,
    CURVE25519_SECRET_LENGTH,
};

pub fn new_public_keys(vault: &mut impl SecretVault) {
    let attributes = SecretAttributes::new(
        SecretType::Curve25519,
        SecretPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH,
    );

    let res = vault.secret_generate(attributes);
    assert!(res.is_ok());
    let p256_ctx_1 = res.unwrap();

    let res = vault.secret_public_key_get(&p256_ctx_1);
    assert!(res.is_ok());
    let pk_1 = res.unwrap();
    assert_eq!(pk_1.as_ref().len(), CURVE25519_PUBLIC_LENGTH);

    let res = vault.secret_generate(attributes);
    assert!(res.is_ok());
    let c25519_ctx_1 = res.unwrap();
    let res = vault.secret_public_key_get(&c25519_ctx_1);
    assert!(res.is_ok());
    let pk_1 = res.unwrap();
    assert_eq!(pk_1.as_ref().len(), CURVE25519_PUBLIC_LENGTH);
}

pub fn new_secret_keys(vault: &mut impl SecretVault) {
    let types = [(SecretType::Curve25519, 32), (SecretType::Buffer, 24)];
    for (t, s) in &types {
        let attributes = SecretAttributes::new(*t, SecretPersistence::Ephemeral, *s);
        let res = vault.secret_generate(attributes);
        assert!(res.is_ok());
        let sk_ctx = res.unwrap();
        let sk = vault.secret_export(&sk_ctx).unwrap();
        assert_eq!(sk.as_ref().len(), *s);
        vault.secret_destroy(sk_ctx).unwrap();
    }
}

pub fn secret_import_export(vault: &mut impl SecretVault) {
    let attributes = SecretAttributes::new(
        SecretType::Curve25519,
        SecretPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH,
    );

    let secret_str = "98d589b0dce92c9e2442b3093718138940bff71323f20b9d158218b89c3cec6e";

    let secret = vault
        .secret_import(decode(secret_str).unwrap().as_slice(), attributes)
        .unwrap();

    assert_eq!(secret.index(), 1);
    assert_eq!(
        encode(vault.secret_export(&secret).unwrap().as_ref()),
        secret_str
    );

    let attributes = SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, 24);
    let secret_str = "5f791cc52297f62c7b8829b15f828acbdb3c613371d21aa1";
    let secret = vault
        .secret_import(decode(secret_str).unwrap().as_slice(), attributes)
        .unwrap();

    assert_eq!(secret.index(), 2);

    assert_eq!(
        encode(vault.secret_export(&secret).unwrap().as_ref()),
        secret_str
    );
}

pub fn secret_attributes_get(vault: &mut impl SecretVault) {
    let attributes = SecretAttributes::new(
        SecretType::Curve25519,
        SecretPersistence::Ephemeral,
        CURVE25519_SECRET_LENGTH,
    );

    let secret = vault.secret_generate(attributes).unwrap();
    assert_eq!(vault.secret_attributes_get(&secret).unwrap(), attributes);

    let attributes = SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, 24);

    let secret = vault.secret_generate(attributes).unwrap();
    assert_eq!(vault.secret_attributes_get(&secret).unwrap(), attributes);
}
