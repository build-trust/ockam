use clap::Args;

use anyhow::anyhow;
use ockam::identity::IdentityIdentifier;
use ockam::{Context, TcpTransport};
use ockam_api::authenticator::direct::types::AddMember;
use ockam_api::config::lookup::ConfigLookup;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelResponse;
use ockam_core::api::Request;
use ockam_multiaddr::{proto, MultiAddr, Protocol};
use tracing::debug;

use crate::node::NodeOpts;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::{help, CommandGlobalOpts, Result};

/// An authorised enroller can add members to a project.
#[derive(Clone, Debug, Args)]
#[clap(hide = help::hide())]
pub struct EnrollCommand {
    /// Orchestrator address to resolve projects present in the `at` argument
    #[clap(flatten)]
    cloud_opts: CloudOpts,

    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(long, short)]
    member: IdentityIdentifier,

    #[clap(long, short)]
    to: MultiAddr,
}

impl EnrollCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(
            |ctx, (opts, cmd)| Runner::new(ctx, opts, cmd).run(),
            (options, self),
        );
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
        let tcp = TcpTransport::create(&self.ctx).await?;
        let map = self.opts.config.get_lookup();
        let to = if let Some(addr) = authority_addr(&self.cmd.to, &map)? {
            debug!(%addr, "establishing secure channel to project authority");
            let mut addr = self.secure_channel(&tcp, &addr).await?;
            for proto in self.cmd.to.iter().skip(1) {
                addr.push_back_value(&proto).map_err(anyhow::Error::from)?
            }
            addr
        } else {
            self.cmd.to.clone()
        };
        let req = Request::post("/members").body(AddMember::new(self.cmd.member.clone()));
        let mut rpc = RpcBuilder::new(&self.ctx, &self.opts, &self.cmd.node_opts.api_node)
            .tcp(&tcp)
            .to(&to)?
            .build()?;
        debug!(addr = %to, member = %self.cmd.member, "requesting to add member");
        rpc.request(req).await?;
        rpc.is_ok()?;
        Ok(())
    }

    async fn secure_channel(
        &self,
        tcp: &TcpTransport,
        addr: &MultiAddr,
    ) -> anyhow::Result<MultiAddr> {
        let mut rpc = RpcBuilder::new(&self.ctx, &self.opts, &self.cmd.node_opts.api_node)
            .tcp(tcp)
            .build()?;
        rpc.request(api::create_secure_channel(addr, None)).await?;
        let res = rpc.parse_response::<CreateSecureChannelResponse>()?;
        let addr = res.addr()?;
        Ok(addr)
    }
}

/// Get the authority address (if any) of the given address.
///
/// If the input address begins with a `/project` protocol we look for the
/// corresponding authority access route and return it.
fn authority_addr(input: &MultiAddr, map: &ConfigLookup) -> anyhow::Result<Option<MultiAddr>> {
    if let Some(proto) = input.first() {
        if proto.code() == proto::Project::CODE {
            let proj = proto.cast::<proto::Project>().expect("project protocol");
            if let Some(p) = map.get_project(&proj) {
                if let Some(r) = &p.authority_access_route {
                    return Ok(Some(r.clone()));
                } else {
                    return Err(anyhow!("missing authority route for project {}", &*proj));
                }
            } else {
                return Err(anyhow!("unknown project {}", &*proj));
            }
        }
    }
    Ok(None)
}
