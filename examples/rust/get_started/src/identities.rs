use ockam::identity::{IdentityChangeConstants, KeyAttributes};
use ockam::Node;
use ockam_core::vault::Secret::Key;
use ockam_core::vault::{SecretAttributes, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH_U32};
use ockam_core::Result;

/// This function can be used to create a new identity and export both it public and private keys
pub async fn create_identity(node: &Node) -> Result<()> {
    let attributes = SecretAttributes::new(
        SecretType::Ed25519,
        SecretPersistence::Persistent,
        CURVE25519_SECRET_LENGTH_U32,
    );
    let key_attributes = KeyAttributes::new(IdentityChangeConstants::ROOT_LABEL.to_string(), attributes);

    let key_id = node.identities_vault().secret_generate(attributes).await?;
    println!("{key_id}");

    if let Key(exported) = node.identities_vault().secret_export(&key_id).await? {
        let s = hex::encode(exported.as_ref());
        println!("secret {s:?}");
    }

    let identity = node
        .identities_creation()
        .create_identity_with_external_key(&key_id, key_attributes)
        .await?;

    println!("identifier {}", identity.identifier());
    println!("history {}", identity.export_hex()?);
    Ok(())
}
