use clap::Args;

use anyhow::{anyhow, Context as _};
use ockam::Context;
use ockam_api::cloud::enroll::auth0::AuthenticateAuth0Token;
use ockam_api::cloud::project::OktaAuth0;
use ockam_core::api::{Request, Status};
use ockam_multiaddr::MultiAddr;
use tracing::{debug, info};

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

/// Authenticate using okta addon
#[derive(Clone, Debug, Args)]
#[command(hide = help::hide())]
pub struct AuthCommand {
    /// Project config file
    #[arg(long = "project", value_name = "PROJECT_JSON_PATH")]
    project: PathBuf,

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

    // Get auth0 token
    let okta_config: OktaAuth0 = p.okta_config.context("Okta addon not configured")?.into();
    let auth0 = Auth0Service::new(Auth0Provider::Okta(okta_config));
    let token = auth0.token().await?;
    // Create secure channel to the project's authority node
    let secure_channel_addr = {
        let authority =
            ProjectAuthority::from_raw(&p.authority_access_route, &p.authority_identity)
                .await?
                .ok_or_else(|| anyhow!("Authority details not configured"))?;
        create_secure_channel_to_authority(&ctx, &opts, &node_name, &authority, authority.address())
            .await?
    };

    // Return address to the "okta_authenticator" worker on the authority node through the secure channel
    let okta_authenticator_addr = {
        let service = MultiAddr::try_from("/service/okta_authenticator")?;
        let mut addr = secure_channel_addr.clone();
        for proto in service.iter() {
            addr.push_back_value(&proto)?;
        }
        addr
    };

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

    // Send enroll request to authority node
    let token = AuthenticateAuth0Token::new(token);
    let req = Request::post("v0/enroll").body(token);
    let mut rpc = RpcBuilder::new(&ctx, &opts, &node_name)
        .to(&okta_authenticator_addr)?
        .build();
    debug!(addr = %okta_authenticator_addr, "enrolling");
    rpc.request(req).await?;
    let (res, dec) = rpc.check_response()?;
    let res = if res.status() == Some(Status::Ok) {
        info!("Enrolled successfully");
        let mut client = Client::new(authenticator_route, &ctx).await?;
        let credential = client.credential().await?;
        println!("{}", credential);
        Ok(())
    } else {
        eprintln!("{}", rpc.parse_err_msg(res, dec));
        Err(anyhow!("Failed to enroll").into())
    };
    delete_embedded_node(&opts.config, &node_name).await;
    res
}
