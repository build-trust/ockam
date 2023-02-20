use ockam::authenticated_storage::InMemoryStorage;
use ockam::identity::{Identity, IdentityStateConst, KeyAttributes};
use ockam::vault::Vault;
use ockam::Context;
use ockam::Result;
use ockam_core::vault::Secret::Key;
use ockam_core::vault::{KeyId, SecretKey, SecretVault};

pub async fn create_identity_with_secret(
    ctx: &Context,
    vault: Vault,
    key_id: &KeyId,
    secret: &str,
) -> Result<Identity<Vault, InMemoryStorage>> {
    let key_attributes = KeyAttributes::default_with_label(IdentityStateConst::ROOT_LABEL);
    vault
        .secret_import(
            Key(SecretKey::new(hex::decode(secret).unwrap())),
            key_attributes.secret_attributes(),
        )
        .await?;
    Identity::create_with_external_key(ctx, &vault, key_id, key_attributes).await
}
