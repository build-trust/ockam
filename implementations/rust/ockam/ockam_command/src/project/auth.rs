use clap::Args;
use std::str::FromStr;

use anyhow::{anyhow, Context as _};
use ockam::identity::credential::OneTimeCode;
use ockam::Context;
use ockam_api::cloud::enroll::auth0::AuthenticateAuth0Token;
use ockam_api::cloud::project::{OktaAuth0, Project};
use ockam_core::api::{Request, Status};
use ockam_multiaddr::MultiAddr;
use tracing::debug;

use crate::enroll::{Auth0Provider, Auth0Service};
use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::ProjectInfo;
use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;

use crate::project::util::create_secure_channel_to_authority;
use ockam_api::authenticator::direct::TokenAcceptorClient;
use ockam_api::config::lookup::ProjectAuthority;
use ockam_api::DefaultAddress;
use ockam_core::route;
use ockam_identity::CredentialsIssuerClient;
use ockam_multiaddr::proto::Service;
use ockam_node::RpcClient;

/// Authenticate with a project node
#[derive(Clone, Debug, Args)]
pub struct AuthCommand {
    #[arg(long = "okta", group = "authentication_method")]
    okta: bool,

    #[arg(long = "token", group = "authentication_method", value_name = "ENROLLMENT TOKEN", value_parser = OneTimeCode::from_str)]
    token: Option<OneTimeCode>,

    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    trust_opts: TrustContextOpts,
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
    let node_name = start_embedded_node(&ctx, &opts, Some(&cmd.trust_opts)).await?;

    let path = match cmd.trust_opts.project_path {
        Some(p) => p,
        None => {
            let default_project = opts
                .state
                .projects
                .default()
                .context("A default project or project parameter is required.")?;
            default_project.path
        }
    };

    // Read (okta and authority) project parameters from project.json
    let s = tokio::fs::read_to_string(path).await?;
    let proj: ProjectInfo = serde_json::from_str(&s)?;
    let project: Project = (&proj).into();

    // Create secure channel to the project's authority node
    // RPC is in embedded mode
    let (secure_channel_addr, _secure_channel_flow_control_id) = if let Some(tc) =
        cmd.trust_opts.trust_context.as_ref()
    {
        let cred_retr = tc.authority()?.own_credential()?;
        let addr = match cred_retr {
            ockam_api::config::cli::CredentialRetrieverConfig::FromCredentialIssuer(c) => {
                &c.multiaddr
            }
            _ => {
                return Err(
                    anyhow!("Trust context must be configured with a credential issuer").into(),
                );
            }
        };
        create_secure_channel_to_authority(
            &ctx,
            &opts,
            &node_name,
            tc.authority()?.identity().await?.identifier().clone(),
            addr,
            Some(cmd.cloud_opts.identity),
        )
        .await?
    } else {
        let authority =
            ProjectAuthority::from_raw(&proj.authority_access_route, &proj.authority_identity)
                .await?
                .ok_or_else(|| anyhow!("Authority details not configured"))?;
        create_secure_channel_to_authority(
            &ctx,
            &opts,
            &node_name,
            authority.identity_id().clone(),
            authority.address(),
            Some(cmd.cloud_opts.identity),
        )
        .await?
    };

    if cmd.okta {
        authenticate_through_okta(&ctx, &opts, &node_name, proj, secure_channel_addr.clone())
            .await?
    } else if let Some(tkn) = cmd.token {
        // Return address to the authenticator in the authority node
        let token_issuer_route = {
            let service = MultiAddr::try_from(
                format!("/service/{}", DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR).as_str(),
            )?;
            let mut addr = secure_channel_addr.clone();
            for proto in service.iter() {
                addr.push_back_value(&proto)?;
            }
            ockam_api::local_multiaddr_to_route(&addr)
                .context(format!("Invalid MultiAddr {addr}"))?
        };
        let client = TokenAcceptorClient::new(
            RpcClient::new(route![DefaultAddress::RPC_PROXY, token_issuer_route], &ctx).await?,
        );
        client.present_token(&tkn).await?
    }

    let credential_issuer_route = {
        let service = MultiAddr::try_from("/service/credential_issuer")?;
        let mut addr = secure_channel_addr.clone();
        for proto in service.iter() {
            addr.push_back_value(&proto)?;
        }
        ockam_api::local_multiaddr_to_route(&addr).context(format!("Invalid MultiAddr {addr}"))?
    };

    let client2 = CredentialsIssuerClient::new(
        route![DefaultAddress::RPC_PROXY, credential_issuer_route],
        &ctx,
    )
    .await?;

    opts.state.projects.create(&project.name, project.clone())?;
    opts.state
        .trust_contexts
        .create(&project.name, project.clone().try_into()?)?;

    let credential = client2.credential().await?;
    println!("---");
    println!("{credential}");
    println!("---");
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
        let service = MultiAddr::try_from(
            format!("/service/{}", DefaultAddress::OKTA_IDENTITY_PROVIDER).as_str(),
        )?;
        let mut addr = secure_channel_addr.clone();
        for proto in service.iter() {
            addr.push_back_value(&proto)?;
        }
        addr.push_front(Service::new(DefaultAddress::RPC_PROXY))?;
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
