use crate::authority_node::{Authority, Configuration};
use crate::CliState;
use ockam_core::Result;
use ockam_node::Context;
use std::collections::BTreeMap;
use tracing::info;

/// Start all the necessary services for an authority node
pub async fn start_node(
    ctx: &Context,
    cli_state: &CliState,
    configuration: &Configuration,
) -> Result<()> {
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

    // start the authenticator services
    authority
        .start_direct_authenticator(ctx, &secure_channel_flow_control_id, configuration)
        .await?;
    debug!("direct authenticator started");

    authority
        .start_enrollment_services(ctx, &secure_channel_flow_control_id, configuration)
        .await?;
    debug!("enrollment services started");

    authority
        .start_credential_issuer(ctx, &secure_channel_flow_control_id, configuration)
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

    // Create an identity for exporting opentelemetry traces
    let exporter = "ockam-opentelemetry-exporter";
    let exporter_identity = match cli_state.get_named_identity(&exporter).await {
        Ok(exporter) => exporter,
        Err(_) => cli_state.create_identity_with_name(&exporter).await?,
    };

    let mut attributes = BTreeMap::new();
    attributes.insert("ockam-relay".to_string(), "ockam-opentelemetry".to_string());
    authority
        .add_member(&exporter_identity.identifier(), &attributes)
        .await?;
    info!("added the ockam-opentelemetry-exporter ({}) identity as a member with the permission to create a relay named ockam-opentelemetry", exporter_identity.identifier());

    debug!("echo service started");

    info!("authority node started");

    Ok(())
}
