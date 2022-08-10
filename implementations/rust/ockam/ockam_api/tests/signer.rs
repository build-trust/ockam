use ockam::identity::authenticated_storage::mem::InMemoryStorage;
use ockam::identity::Identity;
use ockam::vault::Vault;
use ockam_api::auth::types::Attributes;
use ockam_api::signer;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::Context;

#[ockam_macros::test]
async fn signer(ctx: &mut Context) -> Result<()> {
    let v = Vault::create();
    let a = Identity::create(ctx, &v).await?;
    let b = Identity::create(ctx, &v).await?;
    let s = signer::Server::new(Arc::new(a), InMemoryStorage::new());
    ctx.start_worker("signer", s).await?;

    let mut c = signer::Client::new("signer".into(), ctx).await?;

    let bid = b.identifier();
    let mut attrs = Attributes::new();
    attrs.put("id", bid.key_id().as_bytes());
    let cred = c.sign(&attrs).await?.to_owned();
    let atts = c.verify(&cred).await?;
    assert_eq!(attrs, atts);

    ctx.stop().await
}
