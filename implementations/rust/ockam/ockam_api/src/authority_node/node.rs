use crate::authority_node::{Authority, Configuration};
use ockam_core::Result;
use ockam_node::Context;
use tracing::info;

/// Start all the necessary services for an authority node
pub async fn start_node(
    ctx: &Context,
    configuration: &Configuration,
    authority: Authority,
) -> Result<()> {
    debug!("starting authority node");

    debug!("starting services");
    // start a secure channel listener (this also starts a TCP transport)
    let secure_channel_flow_control_id = authority
        .start_secure_channel_listener(ctx, configuration)
        .await?;
    debug!("secure channel listener started");

    // start the authenticator services
    authority.start_direct_authenticator(ctx, &secure_channel_flow_control_id, configuration)?;
    debug!("direct authenticator started");

    authority.start_enrollment_services(ctx, &secure_channel_flow_control_id, configuration)?;
    debug!("enrollment services started");

    authority.start_credential_issuer(ctx, &secure_channel_flow_control_id, configuration)?;
    debug!("credential issuer started");

    // start the Okta service (if the optional configuration has been provided)
    authority.start_okta(ctx, &secure_channel_flow_control_id, configuration)?;
    debug!("okta service started");

    // start an echo service so that the node can be queried as healthy
    authority.start_echo_service(ctx, &secure_channel_flow_control_id)?;

    debug!("echo service started");

    info!("authority node started");

    Ok(())
}
