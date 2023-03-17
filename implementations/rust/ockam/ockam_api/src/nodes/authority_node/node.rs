use crate::nodes::authority_node::{Authority, Configuration};
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
    authority
        .start_secure_channel_listener(ctx, configuration)
        .await?;

    // start the authenticator services
    authority
        .start_direct_authenticator(ctx, configuration)
        .await?;
    authority
        .start_enrollment_services(ctx, configuration)
        .await?;
    authority
        .start_credential_issuer(ctx, configuration)
        .await?;

    // start the Okta service (if the optional configuration has been provided)
    authority.start_okta(ctx, configuration).await?;

    info!(
        "Authority node started with identity\n{}",
        authority.public_identity().await?
    );
    Ok(())
}
