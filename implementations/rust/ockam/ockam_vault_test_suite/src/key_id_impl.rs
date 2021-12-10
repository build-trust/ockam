use ockam_core::hex::decode;
use ockam_core::vault::{
    KeyIdVault, PublicKey, SecretAttributes, SecretPersistence, SecretType, SecretVault,
    CURVE25519_SECRET_LENGTH,
};

pub async fn compute_key_id_for_public_key(vault: &mut impl KeyIdVault) {
    let public =
        decode("68858ea1ea4e1ade755df7fb6904056b291d9781eb5489932f46e32f12dd192a").unwrap();
    let public = PublicKey::new(public.to_vec(), SecretType::X25519);

    let key_id = vault.compute_key_id_for_public_key(&public).await.unwrap();

    assert_eq!(
        key_id,
        "732af49a0b47c820c0a4cac428d6cb80c1fa70622f4a51708163dd87931bc942"
    );
}

pub async fn get_secret_by_key_id(vault: &mut (impl KeyIdVault + SecretVault)) {
    let attributes_set = [
        SecretAttributes::new(
            SecretType::X25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        ),
        SecretAttributes::new(
            SecretType::Ed25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        ),
    ];

    for attributes in attributes_set {
        let secret = vault.secret_generate(attributes).await.unwrap();
        let public = vault.secret_public_key_get(&secret).await.unwrap();

        let key_id = vault.compute_key_id_for_public_key(&public).await.unwrap();
        let secret2 = vault.get_secret_by_key_id(&key_id).await.unwrap();

        assert_eq!(secret.index(), secret2.index());
    }
}
