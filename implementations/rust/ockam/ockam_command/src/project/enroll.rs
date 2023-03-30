use clap::Args;
use ockam_api::cloud::ORCHESTRATOR_RESTART_TIMEOUT;
use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Context as _};
use ockam::identity::IdentityIdentifier;
use ockam::Context;
use ockam_api::authenticator::direct::{DirectAuthenticatorClient, RpcClient, TokenIssuerClient};
use ockam_api::DefaultAddress;
use ockam_multiaddr::MultiAddr;

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::util::create_secure_channel_to_authority;
use crate::util::api::{CloudOpts, CredentialRetrieverConfig, ProjectOpts};
use crate::util::node_rpc;
use crate::{CommandGlobalOpts, Result};

/// An authorised enroller can add members to a project.
#[derive(Clone, Debug, Args)]
pub struct EnrollCommand {
    /// Orchestrator address to resolve projects present in the `at` argument
    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    project_opts: ProjectOpts,

    #[arg(long, short)]
    member: Option<IdentityIdentifier>,

    /// Attributes in `key=value` format to be attached to the member
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    attributes: Vec<String>,
}

impl EnrollCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(
            |ctx, (opts, cmd)| Runner::new(ctx, opts, cmd).run(),
            (options, self),
        );
    }

    fn attributes(&self) -> Result<HashMap<&str, &str>> {
        let mut attributes = HashMap::new();
        for attr in &self.attributes {
            let mut parts = attr.splitn(2, '=');
            let key = parts.next().context("key expected")?;
            let value = parts.next().context("value expected)")?;
            attributes.insert(key, value);
        }
        Ok(attributes)
    }
}

struct Runner {
    ctx: Context,
    opts: CommandGlobalOpts,
    cmd: EnrollCommand,
}

impl Runner {
    fn new(ctx: Context, opts: CommandGlobalOpts, cmd: EnrollCommand) -> Self {
        Self { ctx, opts, cmd }
    }

    async fn run(self) -> Result<()> {
        let node_name =
            start_embedded_node(&self.ctx, &self.opts, Some(&self.cmd.project_opts)).await?;

        let authority_cfg = self
            .cmd
            .project_opts
            .trust_context(self.opts.state.projects.default().ok().map(|p| p.path))
            .unwrap()
            .authority
            .unwrap();

        if let Some(CredentialRetrieverConfig::Online(auth_addr)) =
            &authority_cfg.credential_retriever
        {
            let base_addr = create_secure_channel_to_authority(
                &self.ctx,
                &self.opts,
                &node_name,
                authority_cfg.identity().await.identifier(),
                auth_addr,
                self.cmd.cloud_opts.identity.clone(),
            )
            .await?;
            // If an identity identifier is given add it as a member, otherwise
            // request an enrollment token that a future member can use to get a
            // credential.
            if let Some(id) = &self.cmd.member {
                let direct_authenticator_route = {
                    let service = MultiAddr::try_from(
                        format!("/service/{}", DefaultAddress::DIRECT_AUTHENTICATOR).as_str(),
                    )?;
                    let mut addr = base_addr.clone();
                    for proto in service.iter() {
                        addr.push_back_value(&proto)?;
                    }
                    ockam_api::local_multiaddr_to_route(&addr)
                        .context(format!("Invalid MultiAddr {addr}"))?
                };
                let client = DirectAuthenticatorClient::new(
                    RpcClient::new(direct_authenticator_route, &self.ctx)
                        .await?
                        .with_timeout(Duration::from_secs(ORCHESTRATOR_RESTART_TIMEOUT)),
                );
                client
                    .add_member(id.clone(), self.cmd.attributes()?)
                    .await?
            } else {
                let token_issuer_route = {
                    let service = MultiAddr::try_from(
                        format!("/service/{}", DefaultAddress::ENROLLMENT_TOKEN_ISSUER).as_str(),
                    )?;
                    let mut addr = base_addr.clone();
                    for proto in service.iter() {
                        addr.push_back_value(&proto)?;
                    }
                    ockam_api::local_multiaddr_to_route(&addr)
                        .context(format!("Invalid MultiAddr {addr}"))?
                };
                let client = TokenIssuerClient::new(
                    RpcClient::new(token_issuer_route, &self.ctx)
                        .await?
                        .with_timeout(Duration::from_secs(ORCHESTRATOR_RESTART_TIMEOUT)),
                );
                let token = client.create_token(self.cmd.attributes()?).await?;
                println!("{}", token.to_string())
            }
            delete_embedded_node(&self.opts, &node_name).await;
            Ok(())
        } else {
            Err(anyhow!("An online authority must be configured").into())
        }
    }
}
