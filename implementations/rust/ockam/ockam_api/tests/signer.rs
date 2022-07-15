use ockam::identity::authenticated_storage::mem::InMemoryStorage;
use ockam::identity::Identity;
use ockam::vault::Vault;
use ockam_api::signer;
use ockam_core::Result;
use ockam_node::Context;

#[ockam_macros::test]
async fn signer(ctx: &mut Context) -> Result<()> {
    let v = Vault::create();
    let a = Identity::create(ctx, &v).await?;
    let b = Identity::create(ctx, &v).await?;
    let s = signer::Server::new(a, InMemoryStorage::new());
    ctx.start_worker("signer", s).await?;

    let mut c = signer::Client::new("signer".into(), ctx).await?;

    let cred = c.sign_id(&b.identifier()?).await?.to_owned();
    assert!(c.verify(&cred).await?);

    ctx.stop().await
}
