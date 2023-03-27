use ockam_core::compat::collections::{BTreeMap, HashMap};
use ockam_core::compat::sync::Arc;

use ockam::authenticated_storage::AttributesEntry;
use ockam::identity::Identity;
use ockam::route;
use ockam::vault::Vault;
use ockam_api::authenticator::direct;
use ockam_api::bootstrapped_identities_store::PreTrustedIdentities;
use ockam_core::compat::rand::random_string;
use ockam_core::{AllowAll, Result};
use ockam_identity::credential::Timestamp;
use ockam_identity::{
    PublicIdentity, SecureChannelListenerTrustOptions, SecureChannelTrustOptions,
};
use ockam_node::Context;

#[ockam_macros::test]
async fn credential(ctx: &mut Context) -> Result<()> {
    let api_worker_addr = random_string();
    let auth_worker_addr = random_string();

    let auth_identity = Arc::new(Identity::create(ctx, Vault::create()).await?);
    let member_identity = Arc::new(Identity::create(ctx, Vault::create()).await?);
    let now = Timestamp::now().unwrap();
    let pre_trusted = HashMap::from([(
        member_identity.identifier().clone(),
        AttributesEntry::new(
            BTreeMap::from([("attr".to_string(), "value".as_bytes().to_vec())]),
            now,
            None,
            None,
        ),
    )]);
    let store = Arc::new(PreTrustedIdentities::from(pre_trusted));

    // Create the CredentialIssuer:
    auth_identity
        .create_secure_channel_listener(
            &api_worker_addr,
            SecureChannelListenerTrustOptions::insecure_test(),
        )
        .await?;
    let auth =
        direct::CredentialIssuer::new(b"project42".to_vec(), store, auth_identity.clone()).await?;
    ctx.start_worker(
        &auth_worker_addr,
        auth,
        AllowAll, // In reality there is ABAC rule here.
        AllowAll,
    )
    .await?;

    // Connect to the API channel from the member:
    let e2a = member_identity
        .create_secure_channel(&api_worker_addr, SecureChannelTrustOptions::insecure_test())
        .await?;
    // Add the member via the enroller's connection:
    let c = direct::CredentialIssuerClient::new(
        direct::RpcClient::new(route![e2a.address(), &auth_worker_addr], ctx).await?,
    );
    // Get a fresh member credential and verify its validity:
    let cred = c.credential().await?;
    let exported = auth_identity.export().await?;
    let vault = Vault::create();

    let pkey = PublicIdentity::import(&exported, Vault::create())
        .await
        .unwrap();
    let data = pkey
        .verify_credential(&cred, member_identity.identifier(), vault)
        .await?;
    assert_eq!(
        Some(b"project42".as_slice()),
        data.attributes().get("project_id")
    );
    assert_eq!(Some(b"value".as_slice()), data.attributes().get("attr"));
    ctx.stop().await
}
