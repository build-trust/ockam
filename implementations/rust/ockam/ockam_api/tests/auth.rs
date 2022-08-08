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
    // Create the authority signer service:
    {
        let v = Vault::create();
        let a = Identity::create(ctx, &v).await?;
        let s = signer::Server::new(a, InMemoryStorage::new());
        ctx.start_worker("signer", s).await?;
    }

    // Create the authority authenticator service:
    {
        let e_store = InMemoryStorage::new();
        let m_store = InMemoryStorage::new();
        let admin = direct::Server::admin(m_store.clone(), e_store.clone(), mk_signer(ctx).await?);
        ctx.start_worker("auth-admin", admin).await?;

        let auth = direct::Server::new(m_store, e_store, mk_signer(ctx).await?);
        ctx.start_worker("auth", auth).await?;
    }

    // Create an enroller and add it via the admin service:
    let enroller = {
        let v = Vault::create();
        let e = Identity::create(ctx, &v).await?;
        let mut admin = direct::Client::admin("auth-admin".into(), ctx).await?;
        admin
            .add_enroller(IdentityId::new(e.identifier()?.key_id()))
            .await?;
        e
    };

    // Get the authority's signer key:
    let mut signer_client = mk_signer(ctx).await?;
    let auth_identity = signer_client.identity().await?;

    // Create a secure channel API listener:
    {
        let v = Vault::create();
        let a = Identity::create(ctx, &v).await?;
        a.create_secure_channel_listener("api", TrustEveryonePolicy, &InMemoryStorage::new())
            .await?;
    }

    // Connect to the API channel from the enroller:
    let e2a = enroller
        .create_secure_channel("api", TrustEveryonePolicy, &InMemoryStorage::new())
        .await?;

    // Create a member identity:
    let v = Vault::create();
    let m = Identity::create(ctx, &v).await?;

    // Add the member via the enroller's connection:
    let mut c = direct::Client::new(route![e2a, "auth"], ctx).await?;
    c.add_member(IdentityId::new(m.identifier()?.key_id()))
        .await?;

    // Open a secure channel from member to authenticator:
    let m2a = m
        .create_secure_channel("api", TrustEveryonePolicy, &InMemoryStorage::new())
        .await?;

    let mut c = direct::Client::new(route![m2a, "auth"], ctx).await?;

    // Get a fresh member credential and verify its validity:
    let cred = c.credential().await?.to_owned();
    let atts = c.validate(&cred).await?;
    assert!(atts.get("id").is_some());
    assert!(atts.get("ts").is_some());

    // Here we pretend to be on a different node and want to validate the credential:
    {
        let v = Vault::create();
        let a = Identity::create(ctx, &v).await?;
        let s = signer::Server::new(a, InMemoryStorage::new());
        ctx.start_worker("another-signer", s).await?;
        let mut c = signer::Client::new("another-signer".into(), ctx).await?;
        c.add_identity(auth_identity.identity()).await?;
        assert!(c.verify(&cred).await.is_ok())
    }

    ctx.stop().await
}

async fn mk_signer(ctx: &Context) -> Result<signer::Client> {
    signer::Client::new("signer".into(), ctx).await
}
