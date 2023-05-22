use clap::Args;
use ockam_api::cloud::ORCHESTRATOR_RESTART_TIMEOUT;
use ockam_api::config::cli::TrustContextConfig;
use ockam_api::identity::EnrollmentTicket;
use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Context as _};
use ockam::identity::IdentityIdentifier;
use ockam::Context;
use ockam_api::authenticator::direct::{DirectAuthenticatorClient, TokenIssuerClient};
use ockam_api::config::lookup::{ConfigLookup, ProjectAuthority, ProjectLookup};
use ockam_api::DefaultAddress;
use ockam_core::route;
use ockam_multiaddr::{proto, MultiAddr, Protocol};
use ockam_node::RpcClient;

use crate::identity::get_identity_name;
use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::project::util::create_secure_channel_to_authority;
use crate::util::api::{parse_trust_context, CloudOpts, TrustContextOpts};
use crate::util::node_rpc;
use crate::{CommandGlobalOpts, Result};

/// Add members to a project as an authorised enroller.
#[derive(Clone, Debug, Args)]
pub struct TicketCommand {
    /// Orchestrator address to resolve projects present in the `at` argument
    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    trust_opts: TrustContextOpts,

    #[arg(long, short)]
    member: Option<IdentityIdentifier>,

    #[arg(long, short, default_value = "/project/default")]
    to: MultiAddr,

    /// Attributes in `key=value` format to be attached to the member
    #[arg(short, long = "attribute", value_name = "ATTRIBUTE")]
    attributes: Vec<String>,
}

impl TicketCommand {
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
    cmd: TicketCommand,
}

impl Runner {
    fn new(ctx: Context, opts: CommandGlobalOpts, cmd: TicketCommand) -> Self {
        Self { ctx, opts, cmd }
    }

    async fn run(self) -> Result<()> {
        let node_name =
            start_embedded_node(&self.ctx, &self.opts, Some(&self.cmd.trust_opts)).await?;

        let map = self.opts.config.lookup();
        let mut project: Option<ProjectLookup> = None;
        let mut trust_context: Option<TrustContextConfig> = None;

        let (base_addr, _flow_control_id) =
            if let Some(tc) = self.cmd.trust_opts.trust_context.as_ref() {
                let tc = parse_trust_context(&self.opts.state, tc)?;
                trust_context = Some(tc.clone());
                let cred_retr = tc.authority()?.own_credential()?;
                let addr = match cred_retr {
                    ockam_api::config::cli::CredentialRetrieverConfig::FromCredentialIssuer(c) => {
                        &c.multiaddr
                    }
                    _ => {
                        return Err(anyhow!(
                            "Trust context must be configured with a credential issuer"
                        )
                        .into());
                    }
                };
                let identity = get_identity_name(&self.opts, self.cmd.cloud_opts.identity.clone())?;
                let (sc_addr, sc_flow_control_id) = create_secure_channel_to_authority(
                    &self.ctx,
                    &self.opts,
                    &node_name,
                    tc.authority()?.identity().await?.identifier().clone(),
                    addr,
                    Some(identity),
                )
                .await?;

                (sc_addr, Some(sc_flow_control_id))
            } else if let (Some(p), Some(a)) = get_project(&self.cmd.to, &map)? {
                let identity = get_identity_name(&self.opts, self.cmd.cloud_opts.identity.clone())?;
                let (sc_addr, sc_flow_control_id) = create_secure_channel_to_authority(
                    &self.ctx,
                    &self.opts,
                    &node_name,
                    a.identity_id().clone(),
                    a.address(),
                    Some(identity),
                )
                .await?;

                project = Some(p);
                (sc_addr, Some(sc_flow_control_id))
            } else {
                (self.cmd.to.clone(), None)
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
                RpcClient::new(
                    route![DefaultAddress::RPC_PROXY, direct_authenticator_route],
                    &self.ctx,
                )
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
                RpcClient::new(
                    route![DefaultAddress::RPC_PROXY, token_issuer_route],
                    &self.ctx,
                )
                .await?
                .with_timeout(Duration::from_secs(ORCHESTRATOR_RESTART_TIMEOUT)),
            );
            let token = client.create_token(self.cmd.attributes()?).await?;

            let ticket = EnrollmentTicket::new(token, project, trust_context);
            let ticket_serialized = hex::encode(serde_json::to_vec(&ticket)?);
            print!("{}", ticket_serialized)
        }

        delete_embedded_node(&self.opts, &node_name).await;
        Ok(())
    }
}

/// Get the project authority from the first address protocol.
///
/// If the first protocol is a `/project`, look up the project's config.
fn get_project<'a>(
    input: &MultiAddr,
    map: &'a ConfigLookup,
) -> Result<(Option<ProjectLookup>, Option<&'a ProjectAuthority>)> {
    if let Some(proto) = input.first() {
        if proto.code() == proto::Project::CODE {
            let proj = proto.cast::<proto::Project>().expect("project protocol");
            if let Some(p) = map.get_project(&proj) {
                if let Some(a) = &p.authority {
                    return Ok((Some(p.clone()), Some(a)));
                } else {
                    return Err(anyhow!("missing authority in project {:?}", &*proj).into());
                }
            } else {
                return Err(anyhow!("unknown project {}", &*proj).into());
            }
        }
    }
    Ok((None, None))
}
