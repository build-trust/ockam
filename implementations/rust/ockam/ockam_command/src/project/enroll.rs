use clap::Args;
use ockam_api::cloud::ORCHESTRATOR_RESTART_TIMEOUT;
use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Context as _};
use ockam::identity::IdentityIdentifier;
use ockam::Context;
use ockam_api::authenticator::direct::{DirectAuthenticatorClient, RpcClient, TokenIssuerClient};
use ockam_api::config::lookup::{ConfigLookup, ProjectAuthority};
use ockam_api::DefaultAddress;
use ockam_multiaddr::{proto, MultiAddr, Protocol};

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::node::NodeOpts;
use crate::project::util::create_secure_channel_to_authority;
use crate::util::api::{CloudOpts, ProjectOpts, TrustContextOpts};
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

    #[command(flatten)]
    trust_opts: TrustContextOpts,

    #[command(flatten)]
    node_opts: NodeOpts,

    #[arg(long, short)]
    member: Option<IdentityIdentifier>,

    #[arg(long, short, default_value = "/project/default")]
    to: MultiAddr,

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
        let node_name = start_embedded_node(
            &self.ctx,
            &self.opts,
            Some(&self.cmd.project_opts),
            Some(&self.cmd.trust_opts),
        )
        .await?;

        let map = self.opts.config.lookup();
        let base_addr = if let Some(a) = project_authority(&self.cmd.to, &map)? {
            create_secure_channel_to_authority(
                &self.ctx,
                &self.opts,
                &node_name,
                a,
                a.address(),
                self.cmd.cloud_opts.identity.clone(),
            )
            .await?
        } else {
            self.cmd.to.clone()
        };
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
    }
}

/// Get the project authority from the first address protocol.
///
/// If the first protocol is a `/project`, look up the project's config.
fn project_authority<'a>(
    input: &MultiAddr,
    map: &'a ConfigLookup,
) -> Result<Option<&'a ProjectAuthority>> {
    if let Some(proto) = input.first() {
        if proto.code() == proto::Project::CODE {
            let proj = proto.cast::<proto::Project>().expect("project protocol");
            if let Some(p) = map.get_project(&proj) {
                if let Some(a) = &p.authority {
                    return Ok(Some(a));
                } else {
                    return Err(anyhow!("missing authority in project {:?}", &*proj).into());
                }
            } else {
                return Err(anyhow!("unknown project {}", &*proj).into());
            }
        }
    }
    Ok(None)
}
