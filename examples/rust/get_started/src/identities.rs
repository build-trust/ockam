use ockam::identity::{IdentityChangeConstants, KeyAttributes};
use ockam::Node;
use ockam_core::Result;
use ockam_vault::SecretAttributes;

/// This function can be used to create a new identity and export both it public and private keys
pub async fn create_identity(node: &Node) -> Result<()> {
    let attributes = SecretAttributes::Ed25519;
    let key_attributes = KeyAttributes::new(IdentityChangeConstants::ROOT_LABEL.to_string(), attributes);

    let key_id = node.identities_vault().create_persistent_secret(attributes).await?;
    println!("{key_id}");

    let exported = node
        .identities_vault()
        .get_ephemeral_secret(&key_id, "private key")
        .await?;
    let s = hex::encode(exported.secret().as_ref());
    println!("secret {s:?}");

    let identity = node
        .identities_creation()
        .create_identity_with_external_key(&key_id, key_attributes)
        .await?;

    println!("identifier {}", identity.identifier());
    println!("history {}", identity.export_hex()?);
    Ok(())
}
