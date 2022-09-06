use clap::Args;

use anyhow::anyhow;
use ockam::identity::IdentityIdentifier;
use ockam::{Context, TcpTransport};
use ockam_api::authenticator::direct::types::AddMember;
use ockam_api::config::lookup::{ConfigLookup, ProjectAuthority};
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
        let map = self.opts.config.lookup();
        let to = if let Some(a) = project_authority(&self.cmd.to, &map)? {
            let mut addr = self.secure_channel(&tcp, a).await?;
            for proto in self.cmd.to.iter().skip(1) {
                addr.push_back_value(&proto).map_err(anyhow::Error::from)?
            }
            addr
        } else {
            self.cmd.to.clone()
        };
        let req = Request::post("/members").body(AddMember::new(self.cmd.member.clone()));
        let mut rpc = RpcBuilder::new(&self.ctx, &self.opts, &self.cmd.node_opts.api_node)
            .tcp(&tcp)?
            .to(&to)?
            .build();
        debug!(addr = %to, member = %self.cmd.member, "requesting to add member");
        rpc.request(req).await?;
        rpc.is_ok()?;
        Ok(())
    }

    async fn secure_channel(
        &self,
        tcp: &TcpTransport,
        auth: &ProjectAuthority,
    ) -> anyhow::Result<MultiAddr> {
        let mut rpc = RpcBuilder::new(&self.ctx, &self.opts, &self.cmd.node_opts.api_node)
            .tcp(tcp)?
            .build();
        let addr = replace_project(&self.cmd.to, auth.address())?;
        debug!(%addr, "establishing secure channel to project authority");
        let allowed = vec![auth.identity_id().clone()];
        rpc.request(api::create_secure_channel(&addr, Some(allowed)))
            .await?;
        let res = rpc.parse_response::<CreateSecureChannelResponse>()?;
        let addr = res.addr()?;
        Ok(addr)
    }
}

/// Get the project authority from the first address protocol.
///
/// If the first protocol is a `/project`, look up the project's config.
fn project_authority<'a>(
    input: &MultiAddr,
    map: &'a ConfigLookup,
) -> anyhow::Result<Option<&'a ProjectAuthority>> {
    if let Some(proto) = input.first() {
        if proto.code() == proto::Project::CODE {
            let proj = proto.cast::<proto::Project>().expect("project protocol");
            if let Some(p) = map.get_project(&proj) {
                if let Some(a) = &p.authority {
                    return Ok(Some(a));
                } else {
                    return Err(anyhow!("missing authority in project {:?}", &*proj));
                }
            } else {
                return Err(anyhow!("unknown project {}", &*proj));
            }
        }
    }
    Ok(None)
}

/// Replaces the first `/project` with the given address.
///
/// Assumes (and asserts!) that the first protocol is a `/project`.
fn replace_project(input: &MultiAddr, with: &MultiAddr) -> anyhow::Result<MultiAddr> {
    let mut iter = input.iter();
    let first = iter.next().map(|p| p.code());
    assert_eq!(first, Some(proto::Project::CODE));
    let mut output = MultiAddr::default();
    for proto in with.iter() {
        output.push_back_value(&proto)?
    }
    for proto in iter {
        output.push_back_value(&proto)?
    }
    Ok(output)
}
