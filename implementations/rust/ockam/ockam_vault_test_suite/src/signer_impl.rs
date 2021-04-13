use ockam_vault_core::{
    SecretAttributes, SecretPersistence, SecretType, SecretVault, Signer, Verifier,
    CURVE25519_SECRET_LENGTH,
};

pub fn sign(vault: &mut (impl Signer + Verifier + SecretVault)) {
    let secret = vault
        .secret_generate(SecretAttributes::new(
            SecretType::Curve25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        ))
        .unwrap();
    let res = vault.sign(&secret, b"hello world!");
    assert!(res.is_ok());
    let pubkey = vault.secret_public_key_get(&secret).unwrap();
    let signature = res.unwrap();
    let res = vault.verify(&signature, &pubkey, b"hello world!").unwrap();
    assert!(res);
}
