use clap::Args;

use anyhow::{anyhow, Context as _};
use ockam::Context;
use ockam_api::authenticator::direct::types::OneTimeCode;
use ockam_api::cloud::enroll::auth0::AuthenticateAuth0Token;
use ockam_api::cloud::project::OktaAuth0;
use ockam_core::api::{Request, Status};
use ockam_multiaddr::MultiAddr;
use tracing::debug;

use crate::enroll::{Auth0Provider, Auth0Service};
use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::ProjectInfo;
use crate::util::api::CloudOpts;
use crate::util::{node_rpc, RpcBuilder};
use crate::{help, CommandGlobalOpts};
use std::path::PathBuf;

use crate::project::util::create_secure_channel_to_authority;
use ockam_api::authenticator::direct::Client;
use ockam_api::config::lookup::ProjectAuthority;
use ockam_api::DefaultAddress;

/// Authenticate with a project node
#[derive(Clone, Debug, Args)]
#[command(hide = help::hide())]
pub struct AuthCommand {
    /// Project config file
    #[arg(long = "project", value_name = "PROJECT_JSON_PATH")]
    project: PathBuf,

    #[arg(long = "okta", group = "authentication_method")]
    okta: bool,

    #[arg(long = "token", group = "authentication_method", value_name = "ENROLLMENT TOKEN", value_parser = otc_parser)]
    token: Option<OneTimeCode>,

    #[command(flatten)]
    cloud_opts: CloudOpts,
}

impl AuthCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, AuthCommand),
) -> crate::Result<()> {
    let node_name = start_embedded_node(&ctx, &opts).await?;

    // Read (okta and authority) project parameters from project.json
    let s = tokio::fs::read_to_string(cmd.project).await?;
    let p: ProjectInfo = serde_json::from_str(&s)?;

    // Create secure channel to the project's authority node
    let secure_channel_addr = {
        let authority =
            ProjectAuthority::from_raw(&p.authority_access_route, &p.authority_identity)
                .await?
                .ok_or_else(|| anyhow!("Authority details not configured"))?;
        create_secure_channel_to_authority(
            &ctx,
            &opts,
            &node_name,
            &authority,
            authority.address(),
            cmd.cloud_opts.identity,
        )
        .await?
    };

    if cmd.okta {
        authenticate_through_okta(&ctx, &opts, &node_name, p, secure_channel_addr.clone()).await?
    }
    // If we are given a token to enroll, it's passed on the api call that retrieves the
    // credential the first time,  there is no separate step like in the okta case.
    // After that initial credential request, the identity get enrolled into the authority

    // Return address to the authenticator in the authority node
    let authenticator_route = {
        let service =
            MultiAddr::try_from(format!("/service/{}", DefaultAddress::AUTHENTICATOR).as_str())?;
        let mut addr = secure_channel_addr.clone();
        for proto in service.iter() {
            addr.push_back_value(&proto)?;
        }
        ockam_api::multiaddr_to_route(&addr).context(format!("Invalid MultiAddr {}", addr))?
    };

    let mut client = Client::new(authenticator_route, &ctx).await?;

    let credential = match cmd.token {
        None => client.credential().await?,
        Some(token) => client.credential_with(&token).await?,
    };
    println!("{}", credential);
    delete_embedded_node(&opts, &node_name).await;
    Ok(())
}

async fn authenticate_through_okta(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    p: ProjectInfo<'_>,
    secure_channel_addr: MultiAddr,
) -> crate::Result<()> {
    // Get auth0 token
    let okta_config: OktaAuth0 = p.okta_config.context("Okta addon not configured")?.into();
    let auth0 = Auth0Service::new(Auth0Provider::Okta(okta_config));
    let token = auth0.token().await?;

    // Return address to the "okta_authenticator" worker on the authority node through the secure channel
    let okta_authenticator_addr = {
        let service = MultiAddr::try_from("/service/okta_authenticator")?;
        let mut addr = secure_channel_addr.clone();
        for proto in service.iter() {
            addr.push_back_value(&proto)?;
        }
        addr
    };

    // Send enroll request to authority node
    let token = AuthenticateAuth0Token::new(token);
    let req = Request::post("v0/enroll").body(token);
    let mut rpc = RpcBuilder::new(ctx, opts, node_name)
        .to(&okta_authenticator_addr)?
        .build();
    debug!(addr = %okta_authenticator_addr, "enrolling");
    rpc.request(req).await?;
    let (res, dec) = rpc.check_response()?;
    if res.status() == Some(Status::Ok) {
        Ok(())
    } else {
        eprintln!("{}", rpc.parse_err_msg(res, dec));
        Err(anyhow!("Failed to enroll").into())
    }
}

//TODO: copy-pasted from node/create.rs
fn otc_parser(val: &str) -> anyhow::Result<OneTimeCode> {
    let bytes = hex::decode(val)?;
    let code = <[u8; 32]>::try_from(bytes.as_slice())?;
    Ok(code.into())
}
