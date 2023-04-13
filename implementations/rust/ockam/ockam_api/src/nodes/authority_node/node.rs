use crate::nodes::authority_node::{Authority, Configuration};
use ockam_core::flow_control::FlowControls;
use ockam_core::Result;
use ockam_node::Context;
use tracing::info;

/// Start all the necessary services for an authority node
pub async fn start_node(ctx: &Context, configuration: &Configuration) -> Result<()> {
    // create the authority identity
    // or retrieve it from disk if the node has already been started before
    // The trusted identities in the configuration are used to pre-populate an attribute storage
    // containing those identities and their attributes
    let authority = Authority::create(ctx, configuration).await?;

    // start a secure channel listener (this also starts a TCP transport)
    let flow_controls = FlowControls::default();
    let secure_channel_flow_control_id = authority
        .start_secure_channel_listener(ctx, &flow_controls, configuration)
        .await?;

    // start the authenticator services
    authority
        .start_direct_authenticator(
            ctx,
            &flow_controls,
            &secure_channel_flow_control_id,
            configuration,
        )
        .await?;

    authority
        .start_enrollment_services(
            ctx,
            &flow_controls,
            &secure_channel_flow_control_id,
            configuration,
        )
        .await?;

    authority
        .start_credential_issuer(
            ctx,
            &flow_controls,
            &secure_channel_flow_control_id,
            configuration,
        )
        .await?;

    // start the Okta service (if the optional configuration has been provided)
    authority
        .start_okta(
            ctx,
            &flow_controls,
            &secure_channel_flow_control_id,
            configuration,
        )
        .await?;

    // start an echo service so that the node can be queried as healthy
    authority
        .start_echo_service(ctx, &flow_controls, &secure_channel_flow_control_id)
        .await?;

    info!(
        "Authority node started with identity\n{}",
        authority.public_identity().await?
    );
    Ok(())
}
