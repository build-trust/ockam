use std::collections::HashMap;

use ockam::identity::authenticated_storage::mem::InMemoryStorage;
use ockam::identity::Identity;
use ockam::route;
use ockam::vault::Vault;
use ockam_api::authenticator::direct;
use ockam_api::authenticator::direct::types::Enroller;
use ockam_core::Result;
use ockam_identity::{IdentityIdentifier, PublicIdentity, TrustEveryonePolicy};
use ockam_node::Context;
use tempfile::NamedTempFile;

#[ockam_macros::test]
async fn credential(ctx: &mut Context) -> Result<()> {
    let mut tmpf = NamedTempFile::new().unwrap();
    serde_json::to_writer(&mut tmpf, &HashMap::<IdentityIdentifier, Enroller>::new()).unwrap();

    // Create the authority:
    let authority = {
        let a = Identity::create(ctx, &Vault::create()).await?;
        a.create_secure_channel_listener("api", TrustEveryonePolicy, &InMemoryStorage::new())
            .await?;
        let exported = a.export().await?;
        let store = InMemoryStorage::new();
        let auth = direct::Server::new(b"project42".to_vec(), store, tmpf.path(), a);
        ctx.start_worker("auth", auth).await?;
        exported
    };

    // Create an enroller identity:
    let enroller = Identity::create(ctx, &Vault::create()).await?;

    // Create a member identity:
    let member = Identity::create(ctx, &Vault::create()).await?;

    // Connect to the API channel from the enroller:
    let e2a = enroller
        .create_secure_channel("api", TrustEveryonePolicy, &InMemoryStorage::new())
        .await?;

    // Add the member via the enroller's connection:
    let mut c = direct::Client::new(route![e2a, "auth"], ctx).await?;

    // Enroller is not configured -> fail
    assert!(c.add_member(member.identifier().clone()).await.is_err());

    // Configure enroller
    let enrollers = [(enroller.identifier().clone(), Enroller::default())];
    let mut tmpf = tmpf.reopen().unwrap();
    serde_json::to_writer(&mut tmpf, &HashMap::from(enrollers)).unwrap();
    c.add_member(member.identifier().clone()).await?;

    // Open a secure channel from member to authenticator:
    let m2a = member
        .create_secure_channel("api", TrustEveryonePolicy, &InMemoryStorage::new())
        .await?;

    let mut c = direct::Client::new(route![m2a, "auth"], ctx).await?;

    // Get a fresh member credential and verify its validity:
    let cred = c.credential().await?;
    let pkey = PublicIdentity::import(&authority, &Vault::create())
        .await
        .unwrap();
    let data = cred.verify_signature(&pkey, &Vault::create()).await?;
    assert_eq!(
        Some(b"project42".as_slice()),
        data.attributes().get("project_id")
    );

    ctx.stop().await
}
