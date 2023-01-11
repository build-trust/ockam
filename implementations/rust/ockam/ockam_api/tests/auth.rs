use std::collections::HashMap;

use ockam::authenticated_storage::AuthenticatedStorage;
use ockam::identity::authenticated_storage::mem::InMemoryStorage;
use ockam::identity::Identity;
use ockam::route;
use ockam::vault::Vault;
use ockam_api::authenticator::direct;
use ockam_api::authenticator::direct::types::Enroller;
use ockam_core::compat::rand::random_string;
use ockam_core::{AllowAll, AsyncTryClone, Result};
use ockam_identity::{
    IdentityIdentifier, PublicIdentity, SecureChannelRegistry, TrustEveryonePolicy,
};
use ockam_node::Context;
use tempfile::NamedTempFile;

#[ockam_macros::test]
async fn credential(ctx: &mut Context) -> Result<()> {
    let mut tmpf = NamedTempFile::new().unwrap();
    serde_json::to_writer(&mut tmpf, &HashMap::<IdentityIdentifier, Enroller>::new()).unwrap();

    let api_worker_addr = random_string();
    let auth_worker_addr = random_string();

    let registry = SecureChannelRegistry::new();

    // Create the authority:
    let authority = {
        let a = Identity::create(ctx, &Vault::create()).await?;
        a.create_secure_channel_listener(
            &api_worker_addr,
            TrustEveryonePolicy,
            &InMemoryStorage::new(),
            &registry,
        )
        .await?;
        let store = InMemoryStorage::new();
        let enrollers = tmpf.path().to_str().expect("path should be a string");
        let auth = direct::Server::new(
            b"project42".to_vec(),
            store,
            enrollers,
            a.async_try_clone().await?,
        )?;
        ctx.start_worker(
            &auth_worker_addr,
            auth,
            AllowAll, // Auth checks happen inside the worker
            AllowAll,
        )
        .await?;
        a
    };

    // Create an enroller identity:
    let enroller = Identity::create(ctx, &Vault::create()).await?;

    // Create a member identity:
    let member = Identity::create(ctx, &Vault::create()).await?;

    // Connect to the API channel from the enroller:
    let e2a = enroller
        .create_secure_channel(
            &api_worker_addr,
            TrustEveryonePolicy,
            &InMemoryStorage::new(),
            &registry,
        )
        .await?;

    // Add the member via the enroller's connection:
    let mut c = direct::Client::new(route![e2a.address(), &auth_worker_addr], ctx).await?;

    // Enroller is not configured -> fail
    assert!(c
        .add_member(member.identifier().clone(), HashMap::new())
        .await
        .is_err());
    ctx.stop_worker(&auth_worker_addr).await?;

    // Configure enroller
    let enrollers = [(enroller.identifier().clone(), Enroller::default())];
    let mut tmpfile = tmpf.reopen().unwrap();
    serde_json::to_writer(&mut tmpfile, &HashMap::from(enrollers)).unwrap();

    // Re-create the authority with enroller configured
    let auth_worker_addr = random_string();
    {
        let store = InMemoryStorage::new();
        let enrollers = tmpf.path().to_str().expect("path should be a string");
        let auth = direct::Server::new(
            b"project42".to_vec(),
            store,
            enrollers,
            authority.async_try_clone().await?,
        )?;
        ctx.start_worker(&auth_worker_addr, auth, AllowAll, AllowAll)
            .await?;
    };

    // Re-create client with new auth worker address
    let mut c = direct::Client::new(route![e2a.address(), &auth_worker_addr], ctx).await?;

    let member_attrs = HashMap::from([("role", "member")]);
    c.add_member(member.identifier().clone(), member_attrs)
        .await?;

    // Open a secure channel from member to authenticator:
    let m2a = member
        .create_secure_channel(
            &api_worker_addr,
            TrustEveryonePolicy,
            &InMemoryStorage::new(),
            &registry,
        )
        .await?;

    let mut c = direct::Client::new(route![m2a, &auth_worker_addr], ctx).await?;

    // Get a fresh member credential and verify its validity:
    let cred = c.credential().await?;
    let exported = authority.export().await?;
    let pkey = PublicIdentity::import(&exported, &Vault::create())
        .await
        .unwrap();
    let data = pkey
        .verify_credential(&cred, member.identifier(), &Vault::create())
        .await?;
    assert_eq!(
        Some(b"project42".as_slice()),
        data.attributes().get("project_id")
    );
    assert_eq!(Some(b"member".as_slice()), data.attributes().get("role"));

    ctx.stop().await
}

#[ockam_macros::test]
async fn json_config(ctx: &mut Context) -> Result<()> {
    let api_worker_addr = random_string();
    let auth_worker_addr = random_string();

    let registry = SecureChannelRegistry::new();

    // Create the authority:
    let authority = {
        let a = Identity::create(ctx, &Vault::create()).await?;
        a.create_secure_channel_listener(
            &api_worker_addr,
            TrustEveryonePolicy,
            &InMemoryStorage::new(),
            &registry,
        )
        .await?;
        let store = InMemoryStorage::new();
        let auth = direct::Server::new(
            b"project42".to_vec(),
            store,
            "{}",
            a.async_try_clone().await?,
        )?;
        ctx.start_worker(&auth_worker_addr, auth, AllowAll, AllowAll)
            .await?;
        a
    };

    // Create an enroller identity:
    let enroller = Identity::create(ctx, &Vault::create()).await?;

    // Create a member identity:
    let member = Identity::create(ctx, &Vault::create()).await?;

    // Connect to the API channel from the enroller:
    let e2a = enroller
        .create_secure_channel(
            &api_worker_addr,
            TrustEveryonePolicy,
            &InMemoryStorage::new(),
            &registry,
        )
        .await?;

    // Add the member via the enroller's connection:
    let mut c = direct::Client::new(route![e2a.address(), &auth_worker_addr], ctx).await?;

    // Enroller is not configured -> fail
    assert!(c
        .add_member(member.identifier().clone(), HashMap::new())
        .await
        .is_err());
    ctx.stop_worker(&auth_worker_addr).await?;

    // Configure enroller
    let enrollers = [(enroller.identifier().clone(), Enroller::default())];
    let enrollers_config = serde_json::to_string(&HashMap::from(enrollers)).unwrap();

    // Re-create the authority with enroller configured
    let auth_worker_addr = random_string();
    {
        let store = InMemoryStorage::new();
        let auth = direct::Server::new(
            b"project42".to_vec(),
            store,
            &enrollers_config,
            authority.async_try_clone().await?,
        )?;
        ctx.start_worker(&auth_worker_addr, auth, AllowAll, AllowAll)
            .await?;
    };

    // Re-create client with new auth worker address
    let mut c = direct::Client::new(route![e2a.address(), &auth_worker_addr], ctx).await?;

    // Add member successfully
    let member_attrs = HashMap::from([("role", "member")]);
    c.add_member(member.identifier().clone(), member_attrs)
        .await?;

    // Open a secure channel from member to authenticator:
    let m2a = member
        .create_secure_channel(
            &api_worker_addr,
            TrustEveryonePolicy,
            &InMemoryStorage::new(),
            &registry,
        )
        .await?;

    let mut c = direct::Client::new(route![m2a, &auth_worker_addr], ctx).await?;

    // Get a fresh member credential and verify its validity:
    let cred = c.credential().await?;
    let exported = authority.export().await?;
    let pkey = PublicIdentity::import(&exported, &Vault::create())
        .await
        .unwrap();
    let data = pkey
        .verify_credential(&cred, member.identifier(), &Vault::create())
        .await?;
    assert_eq!(
        Some(b"project42".as_slice()),
        data.attributes().get("project_id")
    );
    assert_eq!(Some(b"member".as_slice()), data.attributes().get("role"));

    ctx.stop().await
}

#[ockam_macros::test]
async fn update_member_format(ctx: &mut Context) -> Result<()> {
    let registry = SecureChannelRegistry::new();

    let mut tmpf = NamedTempFile::new().unwrap();
    serde_json::to_writer(&mut tmpf, &HashMap::<IdentityIdentifier, Enroller>::new()).unwrap();
    // Create the authority:
    let store = InMemoryStorage::new();

    // Create a member identity:
    let member = Identity::create(ctx, &Vault::create()).await?;

    // Member was enrolled, with the old format
    let tru = minicbor::to_vec(true)?;
    store
        .set(member.identifier().key_id(), "member".to_string(), tru)
        .await?;
    let authority = {
        let a = Identity::create(ctx, &Vault::create()).await?;
        a.create_secure_channel_listener(
            "api",
            TrustEveryonePolicy,
            &InMemoryStorage::new(),
            &registry,
        )
        .await?;
        let exported = a.export().await?;
        let enrollers = tmpf.path().to_str().expect("path should be a string");
        let auth = direct::Server::new(b"project42".to_vec(), store, enrollers, a)?;
        ctx.start_worker(
            "auth", auth, AllowAll, // Auth checks happen inside the worker
            AllowAll,
        )
        .await?;
        exported
    };

    // Open a secure channel from member to authenticator:
    let m2a = member
        .create_secure_channel(
            "api",
            TrustEveryonePolicy,
            &InMemoryStorage::new(),
            &registry,
        )
        .await?;

    let mut c = direct::Client::new(route![m2a, "auth"], ctx).await?;

    // Get a fresh member credential and verify its validity. Data is loaded and
    // transformed from the legacy format:
    let cred = c.credential().await?;
    let pkey = PublicIdentity::import(&authority, &Vault::create())
        .await
        .unwrap();
    let data = pkey
        .verify_credential(&cred, member.identifier(), &Vault::create())
        .await?;
    assert_eq!(
        Some(b"project42".as_slice()),
        data.attributes().get("project_id")
    );

    // Get the credential again.  It would have been updated on the store already.
    let cred = c.credential().await?;
    let data = pkey
        .verify_credential(&cred, member.identifier(), &Vault::create())
        .await?;
    assert_eq!(
        Some(b"project42".as_slice()),
        data.attributes().get("project_id")
    );
    assert_eq!(Some(b"member".as_slice()), data.attributes().get("role"));

    ctx.stop().await
}
