use clap::Args;

use anyhow::{anyhow, Context as _};

use ockam::Context;
use ockam_api::cloud::enroll::auth0::AuthenticateAuth0Token;
use ockam_api::cloud::project::{OktaAuth0, Project};
use ockam_api::identity::EnrollmentTicket;
use ockam_core::api::{Request, Status};
use ockam_multiaddr::MultiAddr;
use tracing::debug;

use crate::enroll::{Auth0Provider, Auth0Service};
use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::ProjectInfo;
use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::{CommandGlobalOpts, Result};

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

    #[arg(group = "authentication_method", value_name = "ENROLLMENT TICKET PATH | ENROLLMENT TICKET", value_parser = parse_enroll_ticket)]
    enroll_ticket: Option<EnrollmentTicket>,

    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    trust_opts: TrustContextOpts,
}

fn parse_enroll_ticket(input: &str) -> Result<EnrollmentTicket> {
    let decoded = match std::fs::read_to_string(input) {
        Ok(s) => hex::decode(s)?,
        Err(_) => hex::decode(input)?,
    };

    Ok(serde_json::from_slice(&decoded)?)
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
    let project_as_string: String;

    // Retrieve project info from the enrollment ticket or project.json in the case of okta auth
    let proj: ProjectInfo = if let Some(ticket) = &cmd.enroll_ticket {
        let proj = ticket
            .project()
            .expect("Enrollment ticket is invalid. Ticket does not contain a project.");

        proj.clone().try_into()?
    } else {
        // OKTA AUTHENTICATION FLOW | PREVIOUSLY ENROLLED FLOW
        // currently okta auth does not use an enrollment token
        // however, it could be worked to use one in the future
        //
        // REQUIRES Project passed or default project
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
        project_as_string = tokio::fs::read_to_string(path).await?;
        serde_json::from_str(&project_as_string)?
    };

    let project: Project = (&proj).into();

    // Create secure channel to the project's authority node
    // RPC is in embedded mode
    let (secure_channel_addr, _secure_channel_flow_control_id) = {
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

    if let Some(tkn) = cmd.enroll_ticket {
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
        client.present_token(tkn.one_time_code()).await?
    } else if cmd.okta {
        authenticate_through_okta(&ctx, &opts, &node_name, proj, secure_channel_addr.clone())
            .await?
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
    let token = auth0.token(opts).await?;

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
