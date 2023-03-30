use clap::Args;
use std::str::FromStr;

use anyhow::{anyhow, Context as _};
use ockam::identity::credential::OneTimeCode;
use ockam::Context;
use ockam_api::cloud::enroll::auth0::AuthenticateAuth0Token;
use ockam_api::cloud::project::OktaAuth0;
use ockam_core::api::{Request, Status};
use ockam_multiaddr::MultiAddr;
use tracing::debug;

use crate::enroll::{Auth0Provider, Auth0Service};
use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::util::api::{CloudOpts, CredentialRetrieverConfig, ProjectOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::CommandGlobalOpts;

use crate::project::util::create_secure_channel_to_authority;
use ockam_api::authenticator::direct::{CredentialIssuerClient, RpcClient, TokenAcceptorClient};
use ockam_api::cloud::project::OktaConfig;
use ockam_api::DefaultAddress;

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
    project_opts: ProjectOpts,
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
    let node_name = start_embedded_node(&ctx, &opts, Some(&cmd.project_opts)).await?;

    let trust_context = cmd
        .project_opts
        .trust_context(opts.state.projects.default().ok().map(|p| p.path))
        .context("No trust context configured")
        .unwrap();
    let authority_cfg = trust_context
        .clone()
        .authority
        .context("No authority configured")
        .unwrap();
    if let Some(CredentialRetrieverConfig::Online(auth_addr)) =
        authority_cfg.credential_retriever.as_ref()
    {
        // Create secure channel to the project's authority node
        let secure_channel_addr = create_secure_channel_to_authority(
            &ctx,
            &opts,
            &node_name,
            authority_cfg.identity().await.identifier(),
            auth_addr,
            cmd.cloud_opts.identity.clone(),
        )
        .await?;

        if cmd.okta {
            let okta_config = trust_context.okta_config();
            let okta: Option<OktaConfig> = okta_config
                .as_ref()
                .map(|s| serde_json::from_str(s).unwrap());
            authenticate_through_okta(&ctx, &opts, &node_name, okta, secure_channel_addr.clone())
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
            let client = TokenAcceptorClient::new(RpcClient::new(token_issuer_route, &ctx).await?);
            client.present_token(&tkn).await?
        }

        let credential_issuer_route = {
            let service = MultiAddr::try_from("/service/credential_issuer")?;
            let mut addr = secure_channel_addr.clone();
            for proto in service.iter() {
                addr.push_back_value(&proto)?;
            }
            ockam_api::local_multiaddr_to_route(&addr)
                .context(format!("Invalid MultiAddr {addr}"))?
        };

        let client2 =
            CredentialIssuerClient::new(RpcClient::new(credential_issuer_route, &ctx).await?);

        let credential = client2.credential().await?;
        println!("---");
        println!("{credential}");
        println!("---");
        delete_embedded_node(&opts, &node_name).await;
        Ok(())
    } else {
        Err(anyhow!("An online authority must be configured").into())
    }
}

async fn authenticate_through_okta(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    okta_config: Option<OktaConfig<'_>>,
    secure_channel_addr: MultiAddr,
) -> crate::Result<()> {
    // Get auth0 token
    let okta_config: OktaAuth0 = okta_config.context("Okta addon not configured")?.into();
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
