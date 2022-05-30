use anyhow::anyhow;
use clap::Args;

use ockam::{Context, TcpTransport};
use ockam_api::cloud::enroll::AuthenticatorClient;
use ockam_core::route;
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::{embedded_node, multiaddr_to_route};
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct EnrollCommand {
    /// Ockam's cloud node address
    #[clap(display_order = 1000)]
    pub cloud_addr: MultiAddr,

    #[clap(display_order = 1001, arg_enum, default_value = "auth0")]
    pub authenticator: Authenticator,

    #[clap(display_order = 1002, long, default_value = "default")]
    pub vault: String,

    #[clap(display_order = 1003, long, default_value = "default")]
    pub identity: String,

    #[clap(display_order = 1004, long)]
    pub overwrite: bool,
}

impl EnrollCommand {
    pub fn run(command: EnrollCommand) {
        embedded_node(enroll, command);
    }
}

async fn enroll(mut ctx: Context, command: EnrollCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let _identity = load_or_create_identity(&IdentityOpts::from(&command), &ctx).await?;

    let mut r = multiaddr_to_route(&command.cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", command.cloud_addr))?;
    r = route![r.to_string(), "authenticator"];

    let mut api_client = ockam_api::cloud::Client::new(r, &ctx).await?;
    let auth_client = AuthenticatorClient;
    api_client
        .enroll(&command.authenticator.into(), auth_client)
        .await?;
    println!("Enrolled successfully");

    ctx.stop().await?;
    Ok(())
}

impl<'a> From<&'a EnrollCommand> for IdentityOpts {
    fn from(other: &'a EnrollCommand) -> Self {
        Self {
            overwrite: other.overwrite,
        }
    }
}

#[derive(clap::ArgEnum, Clone, Debug)]
pub enum Authenticator {
    Auth0,
    EnrollmentToken,
}

impl From<Authenticator> for ockam_api::cloud::enroll::Authenticator {
    fn from(other: Authenticator) -> Self {
        match other {
            Authenticator::Auth0 => ockam_api::cloud::enroll::Authenticator::Auth0,
            Authenticator::EnrollmentToken => {
                ockam_api::cloud::enroll::Authenticator::EnrollmentToken
            }
        }
    }
}
