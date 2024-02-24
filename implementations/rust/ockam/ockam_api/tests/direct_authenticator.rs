use crate::common::common::{change_client_identifier, start_authority, AuthorityInfo};
use ockam::identity::secure_channels;
use ockam::identity::utils::now;
use ockam_api::authenticator::direct::Members;
use ockam_api::authenticator::direct::{
    OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY,
};
use ockam_core::Result;
use ockam_node::Context;
use std::collections::BTreeMap;

mod common;

#[ockam_macros::test]
async fn one_admin_test_actions(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo {
        authority_identifier,
        admins,
    } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let now = now()?;

    // Admin is a member itself, Admin can list members
    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0], admin.identifier);

    let members = admin.client.list_members(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    let attrs = members.get(&admin.identifier).unwrap();

    assert!(attrs.added_at().abs_diff(now) < 5.into());
    assert!(attrs.expires_at().is_none());
    assert_eq!(attrs.attested_by(), Some(authority_identifier));

    // Admin cannot delete themself
    let res = admin
        .client
        .delete_member(ctx, admin.identifier.clone())
        .await;
    assert!(res.is_err());

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0], admin.identifier);

    Ok(())
}

#[ockam_macros::test]
async fn admin_cant_delete_admin(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 2).await?;
    let admin1 = &admins[0];
    let admin2 = &admins[1];

    let res = admin1
        .client
        .delete_member(ctx, admin2.identifier.clone())
        .await;
    assert!(res.is_err());
    let res = admin2
        .client
        .delete_member(ctx, admin1.identifier.clone())
        .await;
    assert!(res.is_err());

    let members = admin1.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.contains(&admin1.identifier));
    assert!(members.contains(&admin2.identifier));

    Ok(())
}

#[ockam_macros::test]
async fn admin_can_add_enroller(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let now = now()?;

    let enroller = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes = BTreeMap::<String, String>::default();
    attributes.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
    );
    admin
        .client
        .add_member(ctx, enroller.clone(), attributes.clone())
        .await
        .unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.contains(&enroller));

    let members = admin.client.list_members(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    let attrs = members.get(&enroller).unwrap();

    assert!(attrs.added_at().abs_diff(now) < 5.into());
    assert!(attrs.expires_at().is_none());
    assert_eq!(attrs.attested_by(), Some(admin.identifier.clone()));
    assert_eq!(
        attrs.attrs(),
        &attributes
            .iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
            .collect()
    );

    Ok(())
}

#[ockam_macros::test]
async fn admin_can_delete_enroller(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let enroller = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes = BTreeMap::<String, String>::default();
    attributes.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
    );
    admin
        .client
        .add_member(ctx, enroller.clone(), attributes.clone())
        .await
        .unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.contains(&enroller));

    admin
        .client
        .delete_member(ctx, enroller.clone())
        .await
        .unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    assert!(!members.contains(&enroller));

    Ok(())
}

#[ockam_macros::test]
async fn admin_can_add_member(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let now = now()?;

    let member = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes = BTreeMap::<String, String>::default();
    attributes.insert("KEY".to_string(), "VALUE".to_string());
    admin
        .client
        .add_member(ctx, member.clone(), attributes.clone())
        .await
        .unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.contains(&member));

    let members = admin.client.list_members(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    let attrs = members.get(&member).unwrap();

    assert!(attrs.added_at().abs_diff(now) < 5.into());
    assert!(attrs.expires_at().is_none());
    assert_eq!(attrs.attested_by(), Some(admin.identifier.clone()));
    assert_eq!(
        attrs.attrs(),
        &attributes
            .iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
            .collect()
    );

    Ok(())
}

#[ockam_macros::test]
async fn admin_can_delete_member(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let member = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes = BTreeMap::<String, String>::default();
    attributes.insert("KEY".to_string(), "VALUE".to_string());
    admin
        .client
        .add_member(ctx, member.clone(), attributes.clone())
        .await
        .unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.contains(&member));

    admin
        .client
        .delete_member(ctx, member.clone())
        .await
        .unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    assert!(!members.contains(&member));

    Ok(())
}

#[ockam_macros::test]
async fn enroller_can_list_members(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let enroller = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes_enroller = BTreeMap::<String, String>::default();
    attributes_enroller.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
    );

    let member = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes_member = BTreeMap::<String, String>::default();
    attributes_member.insert("KEY".to_string(), "VALUE".to_string());

    admin
        .client
        .add_member(ctx, enroller.clone(), attributes_enroller.clone())
        .await
        .unwrap();

    admin
        .client
        .add_member(ctx, member.clone(), attributes_member.clone())
        .await
        .unwrap();

    let enroller_client = change_client_identifier(&admin.client, &enroller);

    let members = enroller_client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 3);

    let members = admin.client.list_members(ctx).await.unwrap();
    assert_eq!(members.len(), 3);

    Ok(())
}

#[ockam_macros::test]
async fn enroller_can_add_member(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let now = now()?;

    let enroller = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes_enroller = BTreeMap::<String, String>::default();
    attributes_enroller.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
    );

    let member = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes_member = BTreeMap::<String, String>::default();
    attributes_member.insert("KEY".to_string(), "VALUE".to_string());

    admin
        .client
        .add_member(ctx, enroller.clone(), attributes_enroller.clone())
        .await
        .unwrap();

    let enroller_client = change_client_identifier(&admin.client, &enroller);

    enroller_client
        .add_member(ctx, member.clone(), attributes_member.clone())
        .await
        .unwrap();

    let members = enroller_client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 3);
    assert!(members.contains(&member));

    let members = admin.client.list_members(ctx).await.unwrap();
    assert_eq!(members.len(), 3);
    let attrs = members.get(&member).unwrap();

    assert!(attrs.added_at().abs_diff(now) < 5.into());
    assert!(attrs.expires_at().is_none());
    assert_eq!(attrs.attested_by(), Some(enroller.clone()));
    assert_eq!(
        attrs.attrs(),
        &attributes_member
            .iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
            .collect()
    );

    Ok(())
}

#[ockam_macros::test]
async fn enroller_can_delete_member(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let enroller = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes_enroller = BTreeMap::<String, String>::default();
    attributes_enroller.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
    );
    admin
        .client
        .add_member(ctx, enroller.clone(), attributes_enroller.clone())
        .await
        .unwrap();

    let member = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    admin
        .client
        .add_member(ctx, member.clone(), Default::default())
        .await
        .unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 3);

    let enroller_client = change_client_identifier(&admin.client, &enroller);

    enroller_client
        .delete_member(ctx, member.clone())
        .await
        .unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(!members.contains(&member));

    Ok(())
}

#[ockam_macros::test]
async fn enroller_cant_delete_admin(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let enroller = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes_enroller = BTreeMap::<String, String>::default();
    attributes_enroller.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
    );
    admin
        .client
        .add_member(ctx, enroller.clone(), attributes_enroller.clone())
        .await
        .unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);

    let enroller_client = change_client_identifier(&admin.client, &enroller);

    let res = enroller_client
        .delete_member(ctx, admin.identifier.clone())
        .await;
    assert!(res.is_err());

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.contains(&admin.identifier));

    Ok(())
}

#[ockam_macros::test]
#[ignore] // TODO with admin credentials.  For now, all enrollers have rights to add/remove enrollers
async fn enroller_cant_add_enroller(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let enroller1 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let enroller2 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes_enroller = BTreeMap::<String, String>::default();
    attributes_enroller.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
    );
    admin
        .client
        .add_member(ctx, enroller1.clone(), attributes_enroller.clone())
        .await
        .unwrap();

    let enroller_client = change_client_identifier(&admin.client, &enroller1);

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);

    let res = enroller_client
        .add_member(ctx, enroller2.clone(), attributes_enroller.clone())
        .await;
    assert!(res.is_err());

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);

    Ok(())
}

#[ockam_macros::test]
#[ignore] // TODO with admin credentials.  For now, all enrollers have rights to add/remove enrollers
async fn enroller_cant_delete_enroller(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let enroller1 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let enroller2 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let mut attributes_enroller = BTreeMap::<String, String>::default();
    attributes_enroller.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
    );
    admin
        .client
        .add_member(ctx, enroller1.clone(), attributes_enroller.clone())
        .await
        .unwrap();
    admin
        .client
        .add_member(ctx, enroller2.clone(), attributes_enroller.clone())
        .await
        .unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 3);

    let enroller_client = change_client_identifier(&admin.client, &enroller1);

    let res = enroller_client.delete_member(ctx, enroller1.clone()).await;
    assert!(res.is_err());

    let res = enroller_client.delete_member(ctx, enroller2.clone()).await;
    assert!(res.is_err());

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 3);
    assert!(members.contains(&enroller1));
    assert!(members.contains(&enroller1));

    Ok(())
}

#[ockam_macros::test]
async fn member_cant_do_anything(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let member = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let member2 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    admin
        .client
        .add_member(ctx, member.clone(), Default::default())
        .await
        .unwrap();

    let member_client = change_client_identifier(&admin.client, &member);

    let res = member_client
        .add_member(ctx, member2.clone(), Default::default())
        .await;
    assert!(res.is_err());

    admin
        .client
        .add_member(ctx, member2.clone(), Default::default())
        .await
        .unwrap();

    let res = member_client.delete_member(ctx, member2.clone()).await;
    assert!(res.is_err());

    let res = member_client.list_members(ctx).await;
    assert!(res.is_err());

    let res = member_client.list_member_ids(ctx).await;
    assert!(res.is_err());

    Ok(())
}
