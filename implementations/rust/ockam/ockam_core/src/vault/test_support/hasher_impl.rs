use crate::vault::{Hasher, SecretAttributes, SecretPersistence, SecretType, SecretVault};
use hex::encode;

pub async fn sha256(vault: &mut impl Hasher) {
    let res = vault.sha256(b"a").await;
    assert!(res.is_ok());
    let digest = res.unwrap();
    assert_eq!(
        encode(digest),
        "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
    );
}

pub async fn hkdf(vault: &mut (impl Hasher + SecretVault)) {
    let salt_value = b"hkdf_test";
    let attributes = SecretAttributes::new(
        SecretType::Buffer,
        crate::vault::SecretPersistence::Ephemeral,
        salt_value.len() as u32,
    );
    let salt = vault
        .secret_import(&salt_value[..], attributes)
        .await
        .unwrap();

    let ikm_value = b"a";
    let attributes = SecretAttributes::new(
        SecretType::Buffer,
        SecretPersistence::Ephemeral,
        ikm_value.len() as u32,
    );
    let ikm = vault
        .secret_import(&ikm_value[..], attributes)
        .await
        .unwrap();

    let attributes = SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, 24u32);

    let res = vault
        .hkdf_sha256(&salt, b"", Some(&ikm), vec![attributes])
        .await;
    assert!(res.is_ok());
    let digest = res.unwrap();
    assert_eq!(digest.len(), 1);
    let digest = vault.secret_export(&digest[0]).await.unwrap();
    assert_eq!(
        encode(digest.as_ref()),
        "921ab9f260544b71941dbac2ca2d42c417aa07b53e055a8f"
    );
}
