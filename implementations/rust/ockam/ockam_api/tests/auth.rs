use ockam::identity::authenticated_storage::mem::InMemoryStorage;
use ockam::identity::Identity;
use ockam::route;
use ockam::vault::Vault;
use ockam_api::authenticator::direct;
use ockam_api::signer::{self, types::IdentityId};
use ockam_core::Result;
use ockam_identity::TrustEveryonePolicy;
use ockam_node::Context;

#[ockam_macros::test]
async fn credential(ctx: &mut Context) -> Result<()> {
    let v = Vault::create();

    // Create the authority signer:
    let a = Identity::create(ctx, &v).await?;
    let s = signer::Server::new(a, InMemoryStorage::new());
    ctx.start_worker("signer", s).await?;

    // Create the authenticator admin service:
    let store = InMemoryStorage::new();
    let admin = direct::Server::admin(store.clone(), mk_signer(ctx).await?);
    ctx.start_worker("auth-admin", admin).await?;

    // Create the general authenticator:
    let auth = direct::Server::new(store.clone(), mk_signer(ctx).await?);
    ctx.start_worker("auth", auth).await?;

    // Create an enroller and add it via the admin service:
    let e = Identity::create(ctx, &v).await?;
    let mut admin = direct::Client::admin("auth-admin".into(), ctx).await?;
    admin
        .add_enroller(IdentityId::new(e.identifier()?.key_id()))
        .await?;
    drop(admin);

    // Create a secure channel API listener:
    let a = Identity::create(ctx, &v).await?;
    a.create_secure_channel_listener("api", TrustEveryonePolicy, &InMemoryStorage::new())
        .await?;

    // Connect to the API channel from the enroller:
    let addr1 = e
        .create_secure_channel("api", TrustEveryonePolicy, &InMemoryStorage::new())
        .await?;

    // Create a member identity:
    let m = Identity::create(ctx, &v).await?;

    // Add the member via the enroller's connection:
    let mut c = direct::Client::new(route![addr1, "auth"], ctx).await?;
    c.add_member(IdentityId::new(m.identifier()?.key_id()))
        .await?;

    // Open a secure channel from member to authenticator:
    let addr2 = m
        .create_secure_channel("api", TrustEveryonePolicy, &InMemoryStorage::new())
        .await?;

    let mut c = direct::Client::new(route![addr2, "auth"], ctx).await?;

    // Get a fresh member credential and verify its validity:
    let cred = c.credential().await?.to_owned();
    let atts = c.validate(&cred).await?;
    assert!(atts.get("id").is_some());
    assert!(atts.get("ts").is_some());

    ctx.stop().await
}

async fn mk_signer(ctx: &Context) -> Result<signer::Client> {
    signer::Client::new("signer".into(), ctx).await
}
