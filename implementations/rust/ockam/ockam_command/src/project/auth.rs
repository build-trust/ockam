use clap::Args;

use anyhow::{anyhow, Context as _};
use ockam::Context;
use ockam_api::cloud::enroll::auth0::{Auth0TokenProvider, AuthenticateAuth0Token};
use ockam_api::cloud::project::OktaAuth0;
use ockam_api::nodes::models::secure_channel::{
    CreateSecureChannelResponse, CredentialExchangeMode,
};
use ockam_core::api::{Request, Status};
use ockam_multiaddr::MultiAddr;
use tracing::{debug, info};

use crate::enroll::{Auth0Provider, Auth0Service};
use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::ProjectInfo;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::{help, CommandGlobalOpts};
use std::path::PathBuf;

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
    let node_name = start_embedded_node(&ctx, &opts.config).await?;

    // TODO's
    //  - The secure channel setup is copy-pasted from ockam_command::project::enroll
    //  and should be easier/more direct way to do it.
    //  - The api's okta enroll is not used, remove it

    // Read (okta and authority) project parameters from project.json
    let s = tokio::fs::read_to_string(cmd.project).await?;
    let p: ProjectInfo = serde_json::from_str(&s)?;

    // Get auth0 token
    let okta_config: OktaAuth0 = p.okta_config.context("Okta addon not configured")?.into();
    let auth0 = Auth0Service::new(Auth0Provider::Okta(okta_config));
    let token = auth0.token().await?;

    // Create secure channel to the okta_authenticator service at the project's authority node
    let addr = {
        let authority_access_route: MultiAddr = p
            .authority_access_route
            .context("Authority route not configured")?
            .as_ref()
            .try_into()
            .context("Invalid authority route")?;

        // Return address to the "okta_authenticator" worker on the authority node through a secure channel
        let mut addr = secure_channel(&ctx, &opts, &authority_access_route, &node_name).await?;
        let service = MultiAddr::try_from("/service/okta_authenticator")?;
        for proto in service.iter() {
            addr.push_back_value(&proto)?;
        }
        addr
    };

    // Send enroll request to authority node
    let token = AuthenticateAuth0Token::new(token);
    let req = Request::post("v0/enroll").body(token);
    let mut rpc = RpcBuilder::new(&ctx, &opts, &node_name).to(&addr)?.build();
    debug!(addr = %addr, "enrolling");
    rpc.request(req).await?;
    let (res, dec) = rpc.check_response()?;
    let res = if res.status() == Some(Status::Ok) {
        info!("Enrolled successfully");
        Ok(())
    } else if res.status() == Some(Status::BadRequest) {
        info!("Already enrolled");
        Ok(())
    } else {
        eprintln!("{}", rpc.parse_err_msg(res, dec));
        Err(anyhow!("Failed to enroll").into())
    };
    delete_embedded_node(&opts.config, &node_name).await;
    res
}

async fn secure_channel(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    authority_route: &MultiAddr,
    //authority_identifier: IdentityIdentifier,
    node_name: &str,
) -> anyhow::Result<MultiAddr> {
    let mut rpc = RpcBuilder::new(ctx, opts, node_name).build();
    debug!(%authority_route, "establishing secure channel to project authority");
    //TODO: check authority' identity
    rpc.request(api::create_secure_channel(
        authority_route,
        // Some(allowed),
        None, //Do this means all are ok?
        CredentialExchangeMode::None,
    ))
    .await?;
    let res = rpc.parse_response::<CreateSecureChannelResponse>()?;
    let addr = res.addr()?;
    Ok(addr)
}
