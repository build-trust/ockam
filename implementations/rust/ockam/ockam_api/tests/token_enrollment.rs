use crate::common::common::{change_client_identifier, start_authority, AuthorityInfo};
use ockam::identity::secure_channels;
use ockam::identity::utils::now;
use ockam_api::authenticator::direct::Members;
use ockam_api::authenticator::direct::{
    OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE, OCKAM_ROLE_ATTRIBUTE_KEY,
};
use ockam_api::authenticator::enrollment_tokens::{TokenAcceptor, TokenIssuer};
use ockam_core::Result;
use ockam_node::Context;
use std::collections::BTreeMap;
use std::time::Duration;

mod common;

#[ockam_macros::test]
async fn admin_can_issue_token(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    admin
        .client
        .create_token(ctx, Default::default(), None, None)
        .await
        .unwrap();

    let mut attributes = BTreeMap::<String, String>::default();
    attributes.insert("KEY".to_string(), "VALUE".to_string());
    admin
        .client
        .create_token(ctx, attributes, None, None)
        .await
        .unwrap();

    admin
        .client
        .create_token(ctx, Default::default(), Some(Duration::from_secs(30)), None)
        .await
        .unwrap();

    admin
        .client
        .create_token(ctx, Default::default(), None, Some(5))
        .await
        .unwrap();

    Ok(())
}

#[ockam_macros::test]
async fn admin_can_accept_token(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let otc = admin
        .client
        .create_token(ctx, Default::default(), None, None)
        .await
        .unwrap();

    let res = admin.client.present_token(ctx, otc).await;

    assert!(res.is_ok());
    Ok(())
}

#[ockam_macros::test]
async fn admin_can_add_enroller(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let now = now()?;

    let mut attributes = BTreeMap::<String, String>::default();
    attributes.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
    );
    let otc = admin
        .client
        .create_token(ctx, attributes.clone(), None, None)
        .await
        .unwrap();

    let enroller = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let enroller_client = change_client_identifier(&admin.client, &enroller, None);

    enroller_client.present_token(ctx, otc).await.unwrap();

    let members = enroller_client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    assert!(members.contains(&enroller));

    let members = enroller_client.list_members(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
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
async fn admin_can_add_member(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let now = now()?;

    let mut attributes = BTreeMap::<String, String>::default();
    attributes.insert("KEY".to_string(), "VALUE".to_string());
    let otc = admin
        .client
        .create_token(ctx, attributes.clone(), None, None)
        .await
        .unwrap();

    let member = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let member_client = change_client_identifier(&admin.client, &member, None);

    member_client.present_token(ctx, otc).await.unwrap();

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    assert!(members.contains(&member));

    let members = admin.client.list_members(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
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
async fn enroller_can_add_member(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let now = now()?;

    let mut attributes = BTreeMap::<String, String>::default();
    attributes.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
    );
    let otc = admin
        .client
        .create_token(ctx, attributes.clone(), None, None)
        .await
        .unwrap();

    let enroller = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let enroller_client = change_client_identifier(&admin.client, &enroller, None);

    enroller_client.present_token(ctx, otc).await.unwrap();

    let mut attributes_member = BTreeMap::<String, String>::default();
    attributes_member.insert("KEY".to_string(), "VALUE".to_string());

    let otc = enroller_client
        .create_token(ctx, attributes_member.clone(), None, None)
        .await
        .unwrap();

    let member = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let member_client = change_client_identifier(&admin.client, &member, None);

    member_client.present_token(ctx, otc).await.unwrap();

    let members = enroller_client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.contains(&member));

    let members = enroller_client.list_members(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
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
#[ignore] // TODO with admin credentials.  For now, all enrollers have rights to add/remove enrollers
async fn enroller_cant_add_enroller(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let mut attributes = BTreeMap::<String, String>::default();
    attributes.insert(
        OCKAM_ROLE_ATTRIBUTE_KEY.to_string(),
        OCKAM_ROLE_ATTRIBUTE_ENROLLER_VALUE.to_string(),
    );
    let otc = admin
        .client
        .create_token(ctx, attributes.clone(), None, None)
        .await
        .unwrap();

    let enroller = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let enroller_client = change_client_identifier(&admin.client, &enroller, None);

    enroller_client.present_token(ctx, otc).await.unwrap();

    let res = enroller_client
        .create_token(ctx, attributes.clone(), None, None)
        .await;

    assert!(res.is_err());

    Ok(())
}

#[ockam_macros::test]
async fn token_expiration(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let ttl = 5;
    let otc = admin
        .client
        .create_token(
            ctx,
            Default::default(),
            Some(Duration::from_secs(ttl)),
            None,
        )
        .await
        .unwrap();

    let member = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let member_client = change_client_identifier(&admin.client, &member, None);

    ctx.sleep(Duration::from_secs(ttl + 1)).await;

    let res = member_client.present_token(ctx, otc).await;
    assert!(res.is_err());

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 0);

    Ok(())
}

#[ockam_macros::test]
async fn usage_count_default(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let otc = admin
        .client
        .create_token(ctx, Default::default(), None, None)
        .await
        .unwrap();

    let member1 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let member_client1 = change_client_identifier(&admin.client, &member1, None);

    let member2 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let member_client2 = change_client_identifier(&admin.client, &member2, None);

    member_client1.present_token(ctx, otc).await.unwrap();
    let res = member_client2.present_token(ctx, otc).await;
    assert!(res.is_err());

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 1);
    assert!(members.contains(&member1));
    assert!(!members.contains(&member2));

    Ok(())
}

#[ockam_macros::test]
async fn usage_count2(ctx: &mut Context) -> Result<()> {
    let secure_channels = secure_channels().await?;

    let AuthorityInfo { admins, .. } = start_authority(ctx, secure_channels.clone(), 1).await?;
    let admin = &admins[0];

    let otc = admin
        .client
        .create_token(ctx, Default::default(), None, Some(2))
        .await
        .unwrap();

    let member1 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let member_client1 = change_client_identifier(&admin.client, &member1, None);

    let member2 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let member_client2 = change_client_identifier(&admin.client, &member2, None);

    let member3 = secure_channels
        .identities()
        .identities_creation()
        .create_identity()
        .await?;
    let member_client3 = change_client_identifier(&admin.client, &member3, None);

    member_client1.present_token(ctx, otc).await.unwrap();
    member_client2.present_token(ctx, otc).await.unwrap();
    let res = member_client3.present_token(ctx, otc).await;
    assert!(res.is_err());

    let members = admin.client.list_member_ids(ctx).await.unwrap();
    assert_eq!(members.len(), 2);
    assert!(members.contains(&member1));
    assert!(members.contains(&member2));
    assert!(!members.contains(&member3));

    Ok(())
}
