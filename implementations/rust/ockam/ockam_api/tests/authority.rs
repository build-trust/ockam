use ockam::identity::utils::now;
use ockam::identity::{secure_channels, AttributesEntry, Identifier, SecureChannels};
use ockam::AsyncTryClone;
use ockam_api::authenticator::enrollment_tokens::Members;
use ockam_api::authority_node::{Authority, Configuration};
use ockam_api::bootstrapped_identities_store::PreTrustedIdentities;
use ockam_api::cloud::{AuthorityNode, SecureClients};
use ockam_api::{authority_node, DefaultAddress};
use ockam_core::{Address, Result};
use ockam_multiaddr::MultiAddr;
use ockam_node::Context;
use ockam_transport_tcp::TcpTransport;
use rand::{thread_rng, Rng};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;

#[ockam_macros::test]
async fn authority_starts_with_default_configuration(ctx: &mut Context) -> Result<()> {
    let configuration = default_configuration().await?;

    authority_node::start_node(ctx, &configuration).await?;

    let workers = ctx.list_workers().await?;

    assert!(!workers.contains(&Address::from(DefaultAddress::DIRECT_AUTHENTICATOR)));

    ctx.stop().await?;

    Ok(())
}

#[ockam_macros::test]
async fn controlling_authority_by_member_times_out(ctx: &mut Context) -> Result<()> {
    use std::collections::HashMap;

    let secure_channels = secure_channels();

    let admins = setup(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let member = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?
        .identifier()
        .clone();

    let mut attributes = HashMap::<&str, &str>::default();
    attributes.insert("key", "value");

    admin
        .client
        .add_member(ctx, member.clone(), attributes)
        .await
        .unwrap();

    let authority_node = SecureClients::authority(
        &TcpTransport::create(ctx).await?,
        secure_channels.clone(),
        &admin.identifier,
        &MultiAddr::try_from("/secure/api")?,
        &member,
    )
    .await?;

    // Call from unauthorized Identity will be dropped by incoming ABAC AC, so we won't get
    // any response, we should get a timeout.
    let timeout = Arc::new(AtomicBool::new(true));
    let timeout_clone = timeout.clone();
    let ctx_clone = ctx.async_try_clone().await?;
    ctx.runtime().spawn(async move {
        let _ = authority_node.list_member_ids(&ctx_clone).await;
        timeout_clone.store(false, Ordering::Relaxed)
    });
    ctx.sleep(Duration::from_millis(50)).await;

    assert!(timeout.load(Ordering::Relaxed));

    ctx.stop().await?;

    Ok(())
}

#[ockam_macros::test]
async fn one_admin_test_api(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels();

    let admins = setup(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let now = now()?;

    // Admin is a member itself
    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0], admin.identifier);

    let members = admin.client.list_members(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    let attrs = members.get(&admin.identifier).unwrap();

    assert!(attrs.added() - now < 5.into());
    assert!(attrs.expires().is_none());
    assert!(attrs.attested_by().is_none());

    // Trusted member cannot be deleted
    admin
        .client
        .delete_member(ctx, admin.identifier.clone())
        .await
        .unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0], admin.identifier);

    ctx.stop().await?;

    Ok(())
}

#[ockam_macros::test]
async fn test_one_admin_one_member(ctx: &mut Context) -> Result<()> {
    use std::collections::HashMap;

    let secure_channels = secure_channels();

    let admins = setup(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let now = now()?;

    let member = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?
        .identifier()
        .clone();

    let mut attributes = HashMap::<&str, &str>::default();
    attributes.insert("key", "value");

    admin
        .client
        .add_member(ctx, member.clone(), attributes)
        .await
        .unwrap();

    // Member that we have added + Admin itself
    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.contains(&admin.identifier));
    assert!(members.contains(&member));

    let members = admin.client.list_members(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.get(&admin.identifier).is_some());

    let attrs = members.get(&member).unwrap();

    assert_eq!(attrs.attrs().len(), 2);
    assert_eq!(
        attrs.attrs().get("trust_context_id".as_bytes()),
        Some(&b"123456".to_vec())
    );
    assert_eq!(
        attrs.attrs().get("key".as_bytes()),
        Some(&b"value".to_vec())
    );

    assert!(attrs.added() - now < 5.into());
    assert!(attrs.expires().is_none());
    assert_eq!(attrs.attested_by(), Some(admin.identifier.clone()));

    admin
        .client
        .delete_member(ctx, member.clone())
        .await
        .unwrap();

    // Only Admin left
    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0], admin.identifier);

    let members = admin.client.list_members(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    assert!(members.get(&admin.identifier).is_some());

    ctx.stop().await?;

    Ok(())
}

#[ockam_macros::test]
async fn two_admins_two_members_exist_in_one_global_scope(ctx: &mut Context) -> Result<()> {
    use std::collections::HashMap;

    let secure_channels = secure_channels();

    let admins = setup(ctx, secure_channels.clone(), 2).await?;
    let admin1 = &admins[0];
    let admin2 = &admins[1];

    let member1 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?
        .identifier()
        .clone();
    let member2 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?
        .identifier()
        .clone();

    let mut attributes1 = HashMap::<&str, &str>::default();
    attributes1.insert("key1", "value1");

    let mut attributes2 = HashMap::<&str, &str>::default();
    attributes2.insert("key2", "value2");

    let now = now()?;

    admin1
        .client
        .add_member(ctx, member1.clone(), attributes1)
        .await
        .unwrap();

    admin2
        .client
        .add_member(ctx, member2.clone(), attributes2)
        .await
        .unwrap();

    // Admin1 added Member1, Admin2 added Member2, but both of them see all members:
    // [Admin1, Admin2, Member1, Member2]
    let mut members1 = admin1.client.list_member_ids(ctx).await.unwrap();
    let mut members2 = admin2.client.list_member_ids(ctx).await.unwrap();
    members1.sort();
    members2.sort();
    assert_eq!(members1, members2);
    assert_eq!(members1.len(), 4);
    assert!(members1.contains(&member1));
    assert!(members1.contains(&member2));
    assert!(members1.contains(&admin1.identifier));
    assert!(members1.contains(&admin2.identifier));

    let members1 = admin1.client.list_members(ctx).await.unwrap();
    let members2 = admin2.client.list_members(ctx).await.unwrap();
    assert_eq!(members1, members2);
    assert_eq!(members1.len(), 4);
    assert!(members1.get(&admin1.identifier).is_some());
    assert!(members1.get(&admin2.identifier).is_some());
    let attrs = members1.get(&member1).unwrap();
    assert_eq!(attrs.attrs().len(), 2);
    assert_eq!(
        attrs.attrs().get("trust_context_id".as_bytes()),
        Some(&b"123456".to_vec())
    );
    assert_eq!(
        attrs.attrs().get("key1".as_bytes()),
        Some(&b"value1".to_vec())
    );
    assert!(attrs.added() - now < 5.into());
    assert!(attrs.expires().is_none());
    assert_eq!(attrs.attested_by(), Some(admin1.identifier.clone()));

    let attrs = members1.get(&member2).unwrap();
    assert_eq!(attrs.attrs().len(), 2);
    assert_eq!(
        attrs.attrs().get("trust_context_id".as_bytes()),
        Some(&b"123456".to_vec())
    );
    assert_eq!(
        attrs.attrs().get("key2".as_bytes()),
        Some(&b"value2".to_vec())
    );
    assert!(attrs.added() - now < 5.into());
    assert!(attrs.expires().is_none());
    assert_eq!(attrs.attested_by(), Some(admin2.identifier.clone()));

    // Admin2 added Member2, but Admin1 can also delete Member2
    admin1
        .client
        .delete_member(ctx, member2.clone())
        .await
        .unwrap();
    admin2
        .client
        .delete_member(ctx, member1.clone())
        .await
        .unwrap();

    let mut members1 = admin1.client.list_member_ids(ctx).await.unwrap();
    let mut members2 = admin2.client.list_member_ids(ctx).await.unwrap();
    members1.sort();
    members2.sort();
    assert_eq!(members1, members2);
    assert_eq!(members1.len(), 2);
    assert!(members1.contains(&admin1.identifier));
    assert!(members1.contains(&admin2.identifier));

    let members1 = admin1.client.list_members(ctx).await.unwrap();
    let members2 = admin2.client.list_members(ctx).await.unwrap();
    assert_eq!(members1, members2);
    assert_eq!(members1.len(), 2);
    assert!(members1.get(&admin1.identifier).is_some());
    assert!(members1.get(&admin2.identifier).is_some());

    ctx.stop().await?;

    Ok(())
}

// Default Configuration with fake TrustedIdentifier (which can be changed after the call),
// with freshly created Authority Identifier and temporary files for storage and vault
async fn default_configuration() -> Result<Configuration> {
    let storage_path = NamedTempFile::new().unwrap().keep().unwrap().1;
    let vault_path = NamedTempFile::new().unwrap().keep().unwrap().1;

    let port = thread_rng().gen_range(10000..65535);

    let trusted_identities =
        "{\"I3bab350b6c9ad9c624e54dba4b2e53b2ed95967b\": {\"attribute1\": \"value1\"}}";

    let trusted_identities = PreTrustedIdentities::new_from_string(trusted_identities)?;

    let mut configuration = authority_node::Configuration {
        identifier: "I4dba4b2e53b2ed95967b3bab350b6c9ad9c624e5".try_into()?,
        storage_path,
        vault_path,
        project_identifier: "123456".to_string(),
        tcp_listener_address: format!("127.0.0.1:{}", port),
        secure_channel_listener_name: None,
        authenticator_name: None,
        trusted_identities,
        no_direct_authentication: true,
        no_token_enrollment: true,
        okta: None,
    };

    // Hack to create Authority Identity using the same vault and storage
    let authority_sc_temp = Authority::create(&configuration).await?.secure_channels();

    let authority_identity = authority_sc_temp
        .identities()
        .identities_creation()
        .create_identity()
        .await?;

    configuration.identifier = authority_identity.identifier().clone();

    Ok(configuration)
}

struct Admin {
    identifier: Identifier,
    client: AuthorityNode,
}

// Start an Authority with given number of freshly generated Admins, also instantiate a Client for
// each of the admins
async fn setup(
    ctx: &Context,
    secure_channels: Arc<SecureChannels>,
    number_of_admins: usize,
) -> Result<Vec<Admin>> {
    use ockam_core::compat::collections::HashMap;
    let now = now()?;

    let mut admin_ids = vec![];

    let mut trusted_identities = HashMap::<Identifier, AttributesEntry>::new();

    let mut attrs = BTreeMap::<Vec<u8>, Vec<u8>>::new();
    attrs.insert(b"ockam-role".to_vec(), b"enroller".to_vec());
    attrs.insert(b"trust_context_id".to_vec(), b"123456".to_vec());

    for _ in 0..number_of_admins {
        let admin = secure_channels
            .identities()
            .identities_creation()
            .create_identity()
            .await?
            .identifier()
            .clone();

        let entry = AttributesEntry::new(attrs.clone(), now, None, None);
        trusted_identities.insert(admin.clone(), entry);

        admin_ids.push(admin);
    }

    let mut configuration = default_configuration().await?;

    configuration.no_direct_authentication = false;

    configuration.trusted_identities = PreTrustedIdentities::Fixed(trusted_identities);

    authority_node::start_node(ctx, &configuration).await?;

    let mut admins = vec![];
    for admin_id in admin_ids {
        let authority_node = SecureClients::authority(
            &TcpTransport::create(ctx).await?,
            secure_channels.clone(),
            &configuration.identifier,
            &MultiAddr::try_from("/secure/api")?,
            &admin_id,
        )
        .await?;

        admins.push(Admin {
            identifier: admin_id,
            client: authority_node,
        });
    }

    Ok(admins)
}
