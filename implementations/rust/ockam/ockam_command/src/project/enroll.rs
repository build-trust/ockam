use clap::Args;
use std::collections::HashMap;

use anyhow::{anyhow, Context as _};
use ockam::identity::IdentityIdentifier;
use ockam::Context;
use ockam_api::authenticator::direct::types::{AddMember, CreateToken, OneTimeCode};
use ockam_api::config::lookup::{ConfigLookup, ProjectAuthority, ProjectLookup};
use ockam_core::api::Request;
use ockam_multiaddr::{proto, MultiAddr, Protocol};
use tracing::debug;

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::node::NodeOpts;
use crate::project::util::create_secure_channel_to_authority;
use crate::util::api::{CloudOpts, ProjectOpts};
use crate::util::{node_rpc, RpcBuilder};
use crate::{CommandGlobalOpts, Result};

use super::ProjectInfo;

/// An authorised enroller can add members to a project.
#[derive(Clone, Debug, Args)]
#[command(arg_required_else_help = true)]
pub struct EnrollCommand {
    /// Orchestrator address to resolve projects present in the `at` argument
    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    project_opts: ProjectOpts,

    #[command(flatten)]
    node_opts: NodeOpts,

    #[arg(long, short)]
    member: Option<IdentityIdentifier>,

    #[arg(long, short, default_value = "/project/default/service/authenticator")]
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

    fn attributes(&self) -> Result<HashMap<String, String>> {
        let mut attributes = HashMap::new();
        for attr in &self.attributes {
            let mut parts = attr.splitn(2, '=');
            let key = parts.next().context("key expected")?;
            let value = parts.next().context("value expected)")?;
            attributes.insert(key.to_string(), value.to_string());
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

        let map = self.opts.config.lookup();
        let to = if let Some(proj_path) = &self.cmd.project_opts.project_path {
            let s = tokio::fs::read_to_string(proj_path).await?;
            let proj_info: ProjectInfo = serde_json::from_str(&s)?;
            let project_lookup = ProjectLookup::from_project(&(&proj_info).into()).await?;
            if let Some(a) = &project_lookup.authority {
                let mut addr = create_secure_channel_to_authority(
                    &self.ctx,
                    &self.opts,
                    &node_name,
                    a,
                    &replace_project(&self.cmd.to, a.address())?,
                    None, //for now always the default identity
                )
                .await?;
                for proto in self.cmd.to.iter().skip(1) {
                    addr.push_back_value(&proto).map_err(anyhow::Error::from)?
                }
                addr
            } else {
                return Err(anyhow!("Provided project is missing the authority details").into());
            }
        } else if let Some(a) = project_authority(&self.cmd.to, &map)? {
            let mut addr = create_secure_channel_to_authority(
                &self.ctx,
                &self.opts,
                &node_name,
                a,
                &replace_project(&self.cmd.to, a.address())?,
                None, //for now always the default identity
            )
            .await?;
            for proto in self.cmd.to.iter().skip(1) {
                addr.push_back_value(&proto).map_err(anyhow::Error::from)?
            }
            addr
        } else {
            self.cmd.to.clone()
        };
        let mut rpc = RpcBuilder::new(&self.ctx, &self.opts, &node_name)
            .to(&to)?
            .build();

        // If an identity identifier is given add it as a member, otherwise
        // request an enrollment token that a future member can use to get a
        // credential.
        if let Some(id) = &self.cmd.member {
            debug!(addr = %to, member = %id, attrs = ?self.cmd.attributes, "requesting to add member");
            let req = Request::post("/members")
                .body(AddMember::new(id.clone()).with_attributes(self.cmd.attributes()?));
            rpc.request(req).await?;
            rpc.is_ok()?;
        } else {
            debug!(addr = %to, attrs = ?self.cmd.attributes, "requesting token");
            let req = Request::post("/tokens")
                .body(CreateToken::new().with_attributes(self.cmd.attributes()?));
            rpc.request(req).await?;
            let res: OneTimeCode = rpc.parse_response()?;
            println!("{}", hex::encode(res.code()))
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
pub fn replace_project(input: &MultiAddr, with: &MultiAddr) -> anyhow::Result<MultiAddr> {
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
