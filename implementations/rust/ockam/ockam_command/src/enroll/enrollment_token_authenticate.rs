use anyhow::anyhow;
use clap::Args;

use ockam::{Context, TcpTransport};
use ockam_api::cloud::enroll::enrollment_token::EnrollmentToken;
use ockam_api::cloud::enroll::Token;

use crate::enroll::EnrollCommand;
use crate::old::identity::load_or_create_identity;
use crate::util::embedded_node;

#[derive(Clone, Debug, Args)]
pub struct AuthenticateEnrollmentTokenCommand;

impl AuthenticateEnrollmentTokenCommand {
    pub fn run(command: EnrollCommand) {
        embedded_node(authenticate, command);
    }
}

async fn authenticate(mut ctx: Context, cmd: EnrollCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let identity = load_or_create_identity(&ctx, cmd.identity_opts.overwrite).await?;

    let route = ockam_api::multiaddr_to_route(&cmd.address)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cmd.address))?;

    let token = cmd.token.ok_or_else(|| anyhow!("Token was not passed"))?;

    let mut api_client = ockam_api::cloud::MessagingClient::new(route, identity, &ctx).await?;
    api_client
        .authenticate_enrollment_token(EnrollmentToken::new(Token(token.into())))
        .await?;
    println!("Token authenticated");

    ctx.stop().await?;
    Ok(())
}
