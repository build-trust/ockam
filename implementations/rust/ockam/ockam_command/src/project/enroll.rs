use clap::Args;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use std::sync::Arc;

use ockam::Context;
use ockam_api::authenticator::direct::TokenAcceptorClient;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::{OktaAuth0, Project};
use ockam_api::config::lookup::ProjectAuthority;
use ockam_api::identity::EnrollmentTicket;
use ockam_api::DefaultAddress;
use ockam_core::route;
use ockam_identity::CredentialsIssuerClient;
use ockam_multiaddr::proto::Service;
use ockam_multiaddr::MultiAddr;
use ockam_node::RpcClient;

use crate::enroll::{enroll_with_node, OidcService, OktaOidcProvider};
use crate::identity::{get_identity_name, initialize_identity_if_default};
use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::util::create_secure_channel_to_authority;
use crate::project::ProjectInfo;
use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/enroll/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/enroll/after_long_help.txt");

/// Use an OTC to enroll an identity with a project node
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct EnrollCommand {
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

impl EnrollCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_identity_if_default(&opts, &self.cloud_opts.identity);
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, EnrollCommand),
) -> miette::Result<()> {
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
        let path = match cmd.trust_opts.project_path.as_ref() {
            Some(p) => p.clone(),
            None => {
                let default_project = opts
                    .state
                    .projects
                    .default()
                    .context("A default project or project parameter is required.")?;
                default_project.path().clone()
            }
        };

        // Read (okta and authority) project parameters from project.json
        project_as_string = tokio::fs::read_to_string(path).await.into_diagnostic()?;
        serde_json::from_str(&project_as_string).into_diagnostic()?
    };

    let project: Project = (&proj).into();

    // Create secure channel to the project's authority node
    // RPC is in embedded mode
    let secure_channel_addr = {
        let authority =
            ProjectAuthority::from_raw(&proj.authority_access_route, &proj.authority_identity)
                .await?
                .ok_or_else(|| miette!("Authority details not configured"))?;
        let identity = get_identity_name(&opts.state, &cmd.cloud_opts.identity);
        create_secure_channel_to_authority(
            &ctx,
            &opts,
            &node_name,
            authority.identity_id().clone(),
            authority.address(),
            Some(identity),
        )
        .await?
    };

    if let Some(tkn) = cmd.enroll_ticket.as_ref() {
        // Return address to the authenticator in the authority node
        let token_issuer_route = {
            let service = MultiAddr::try_from(
                format!("/service/{}", DefaultAddress::ENROLLMENT_TOKEN_ACCEPTOR).as_str(),
            )
            .into_diagnostic()?;
            let mut addr = secure_channel_addr.clone();
            for proto in service.iter() {
                addr.push_back_value(&proto).into_diagnostic()?;
            }
            ockam_api::local_multiaddr_to_route(&addr).ok_or(miette!("Invalid MultiAddr {addr}"))?
        };
        let client = TokenAcceptorClient::new(
            RpcClient::new(route![DefaultAddress::RPC_PROXY, token_issuer_route], &ctx)
                .await
                .into_diagnostic()?,
        );
        client
            .present_token(tkn.one_time_code())
            .await
            .into_diagnostic()?
    } else if cmd.okta {
        authenticate_through_okta(&ctx, &opts, &node_name, proj, secure_channel_addr.clone())
            .await?
    }

    let credential_issuer_route = {
        let service = MultiAddr::try_from("/service/credential_issuer").into_diagnostic()?;
        let mut addr = secure_channel_addr.clone();
        for proto in service.iter() {
            addr.push_back_value(&proto).into_diagnostic()?;
        }
        ockam_api::local_multiaddr_to_route(&addr).ok_or(miette!("Invalid MultiAddr {addr}"))?
    };

    let client2 = CredentialsIssuerClient::new(
        route![DefaultAddress::RPC_PROXY, credential_issuer_route],
        &ctx,
    )
    .await
    .into_diagnostic()?;

    opts.state
        .projects
        .overwrite(&project.id, project.clone())?;
    opts.state
        .trust_contexts
        .overwrite(&project.id, project.clone().try_into()?)?;

    let credential = client2.credential().await.into_diagnostic()?;
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
) -> miette::Result<()> {
    // Get auth0 token
    let okta_config: OktaAuth0 = p
        .okta_config
        .ok_or(miette!("Okta addon not configured"))?
        .into();
    let auth0 = OidcService::new(Arc::new(OktaOidcProvider::new(okta_config)));
    let token = auth0.get_token_interactively(opts).await?;

    // Return address to the "okta_authenticator" worker on the authority node through the secure channel
    let okta_authenticator_addr = {
        let service = MultiAddr::try_from(
            format!("/service/{}", DefaultAddress::OKTA_IDENTITY_PROVIDER).as_str(),
        )
        .into_diagnostic()?;
        let mut addr = secure_channel_addr.clone();
        for proto in service.iter() {
            addr.push_back_value(&proto).into_diagnostic()?;
        }
        addr.push_front(Service::new(DefaultAddress::RPC_PROXY))
            .into_diagnostic()?;
        addr
    };

    enroll_with_node(ctx, opts, &okta_authenticator_addr, node_name, token)
        .await
        .wrap_err("Failed to enroll your local identity with Ockam Orchestrator")
}
