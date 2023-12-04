use crate::common::common::default_configuration;
use ockam_api::authority_node;
use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_core::{Address, Result};
use ockam_node::Context;

mod common;

#[ockam_macros::test]
async fn authority_starts_with_default_configuration(ctx: &mut Context) -> Result<()> {
    let configuration = default_configuration().await?;

    authority_node::start_node(ctx, &configuration).await?;

    let workers = ctx.list_workers().await?;

    assert!(!workers.contains(&Address::from(DefaultAddress::DIRECT_AUTHENTICATOR)));
    assert!(!workers.contains(&Address::from(DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR)));
    assert!(!workers.contains(&Address::from(DefaultAddress::ENROLLMENT_TOKEN_ISSUER)));
    assert!(workers.contains(&Address::from(DefaultAddress::CREDENTIAL_ISSUER)));
    assert!(workers.contains(&Address::from(DefaultAddress::SECURE_CHANNEL_LISTENER)));
    assert!(workers.contains(&Address::from(DefaultAddress::ECHO_SERVICE)));

    Ok(())
}

#[ockam_macros::test]
async fn authority_starts_direct_authenticator(ctx: &mut Context) -> Result<()> {
    let mut configuration = default_configuration().await?;

    configuration.no_direct_authentication = false;

    authority_node::start_node(ctx, &configuration).await?;

    let workers = ctx.list_workers().await?;

    assert!(workers.contains(&Address::from(DefaultAddress::DIRECT_AUTHENTICATOR)));
    assert!(!workers.contains(&Address::from(DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR)));
    assert!(!workers.contains(&Address::from(DefaultAddress::ENROLLMENT_TOKEN_ISSUER)));
    assert!(workers.contains(&Address::from(DefaultAddress::CREDENTIAL_ISSUER)));
    assert!(workers.contains(&Address::from(DefaultAddress::SECURE_CHANNEL_LISTENER)));
    assert!(workers.contains(&Address::from(DefaultAddress::ECHO_SERVICE)));

    Ok(())
}

#[ockam_macros::test]
async fn authority_starts_enrollment_token(ctx: &mut Context) -> Result<()> {
    let mut configuration = default_configuration().await?;

    configuration.no_token_enrollment = false;

    authority_node::start_node(ctx, &configuration).await?;

    let workers = ctx.list_workers().await?;

    assert!(!workers.contains(&Address::from(DefaultAddress::DIRECT_AUTHENTICATOR)));
    assert!(workers.contains(&Address::from(DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR)));
    assert!(workers.contains(&Address::from(DefaultAddress::ENROLLMENT_TOKEN_ISSUER)));
    assert!(workers.contains(&Address::from(DefaultAddress::CREDENTIAL_ISSUER)));
    assert!(workers.contains(&Address::from(DefaultAddress::SECURE_CHANNEL_LISTENER)));
    assert!(workers.contains(&Address::from(DefaultAddress::ECHO_SERVICE)));

    Ok(())
}
