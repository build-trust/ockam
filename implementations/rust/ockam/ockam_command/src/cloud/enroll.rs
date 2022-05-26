use anyhow::anyhow;
use clap::Args;

use ockam_api::cloud::enroll::AuthenticatorClient;
use ockam_core::route;
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::multiaddr_to_route;
use crate::IdentityOpts;

pub async fn run(args: EnrollCommandArgs, ctx: &mut ockam::Context) -> anyhow::Result<()> {
    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let _identity = load_or_create_identity(&IdentityOpts::from(&args), ctx).await?;

    let addr = multiaddr_to_route(&args.cloud_addr)
        .ok_or_else(|| anyhow!("failed to parse address: {}", args.cloud_addr))?;
    let route = route![addr.to_string(), "authenticator"];
    let mut api_client = ockam_api::cloud::Client::new(route, ctx).await?;
    let auth_client = AuthenticatorClient;
    api_client
        .enroll(&args.authenticator.into(), auth_client)
        .await?;
    println!("Enrolled successfully");
    Ok(())
}

#[derive(Clone, Debug, Args)]
pub struct EnrollCommandArgs {
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

impl<'a> From<&'a EnrollCommandArgs> for IdentityOpts {
    fn from(other: &'a EnrollCommandArgs) -> Self {
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
