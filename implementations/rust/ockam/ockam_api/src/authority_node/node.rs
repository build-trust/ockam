use crate::authenticator::{InMemoryMembersStorage, Member, MembersStorage};
use crate::authority_node::{Authority, Configuration};
use ockam_core::Result;
use ockam_node::Context;
use std::sync::Arc;
use tracing::info;

/// Start all the necessary services for an authority node
pub async fn start_node(ctx: &Context, configuration: &Configuration) -> Result<()> {
    debug!("starting authority node");
    // create the authority identity
    // or retrieve it from disk if the node has already been started before
    // The trusted identities in the configuration are used to pre-populate an attribute storage
    // containing those identities and their attributes
    let authority = Authority::create(configuration).await?;

    debug!("starting services");
    // start a secure channel listener (this also starts a TCP transport)
    let secure_channel_flow_control_id = authority
        .start_secure_channel_listener(ctx, configuration)
        .await?;
    debug!("secure channel listener started");

    // FIXME: Replace with persistent storage
    let members_storage = Arc::new(InMemoryMembersStorage::new());

    // TODO: More work required here to identify the proper behaviour
    //  What should happen if we provide different trusted_identity on the next run?
    //  Should we delete the old ones?
    for (identifier, entry) in configuration.trusted_identities.map() {
        let member = Member::new(
            identifier.clone(),
            entry.attrs().clone(),
            None,
            entry.added(),
            true,
        );
        members_storage.add_member(member).await?;
    }

    // start the authenticator services
    authority
        .start_direct_authenticator(
            ctx,
            &secure_channel_flow_control_id,
            configuration,
            members_storage.clone(),
        )
        .await?;
    debug!("direct authenticator started");

    authority
        .start_enrollment_services(
            ctx,
            &secure_channel_flow_control_id,
            configuration,
            members_storage.clone(),
        )
        .await?;
    debug!("enrollment services started");

    authority
        .start_credential_issuer(ctx, &secure_channel_flow_control_id, members_storage)
        .await?;
    debug!("credential issuer started");

    // start the Okta service (if the optional configuration has been provided)
    authority
        .start_okta(ctx, &secure_channel_flow_control_id, configuration)
        .await?;
    debug!("okta service started");

    // start an echo service so that the node can be queried as healthy
    authority
        .start_echo_service(ctx, &secure_channel_flow_control_id)
        .await?;
    debug!("echo service started");

    info!("authority node started");
    Ok(())
}
