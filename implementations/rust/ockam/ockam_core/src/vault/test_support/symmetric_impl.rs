use crate::vault::{
    KeyAttributes, KeyPersistence, KeyType, KeyVault, SymmetricVault, AES128_SECRET_LENGTH_U32,
};

pub async fn encryption(vault: &mut (impl SymmetricVault + KeyVault)) {
    let message = b"Ockam Test Message";
    let nonce = b"TestingNonce";
    let aad = b"Extra payload data";
    let attributes = KeyAttributes::new(
        KeyType::Aes,
        KeyPersistence::Ephemeral,
        AES128_SECRET_LENGTH_U32,
    );

    let ctx = &vault.generate_key(attributes).await.unwrap();
    let res = vault
        .aead_aes_gcm_encrypt(ctx, message.as_ref(), nonce.as_ref(), aad.as_ref())
        .await;
    assert!(res.is_ok());
    let mut ciphertext = res.unwrap();
    let res = vault
        .aead_aes_gcm_decrypt(ctx, ciphertext.as_slice(), nonce.as_ref(), aad.as_ref())
        .await;
    assert!(res.is_ok());
    let plaintext = res.unwrap();
    assert_eq!(plaintext, message.to_vec());
    ciphertext[0] ^= 0xb4;
    ciphertext[1] ^= 0xdc;
    let res = vault
        .aead_aes_gcm_decrypt(ctx, ciphertext.as_slice(), nonce.as_ref(), aad.as_ref())
        .await;
    assert!(res.is_err());
}
