use clap::{Args, Subcommand};
use colorful::Colorful;
use miette::miette;
use minicbor::Encode;

use ockam::Context;
use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;

use crate::node::NodeOpts;
use crate::terminal::OckamColor;
use crate::util::{api, node_rpc};
use crate::{fmt_ok, CommandGlobalOpts};
use crate::{fmt_warn, Result};

/// Start a specified service
#[derive(Clone, Debug, Args)]
pub struct StartCommand {
    #[command(subcommand)]
    pub create_subcommand: StartSubCommand,
    #[command(flatten)]
    pub node_opts: NodeOpts,
}

#[derive(Clone, Debug, Subcommand)]
pub enum StartSubCommand {
    Hop {
        #[arg(long, default_value_t = hop_default_addr())]
        addr: String,
    },
}

fn hop_default_addr() -> String {
    DefaultAddress::HOP_SERVICE.to_string()
}

impl StartCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(opts.rt.clone(), rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, StartCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: StartCommand) -> miette::Result<()> {
    let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.node_opts.at_node).await?;
    let is_hop_service = true;
    let addr = match cmd.create_subcommand {
        StartSubCommand::Hop { addr, .. } => {
            start_hop_service(ctx, &node, &addr).await?;
            addr
        }
    };

    opts.terminal.write_line(&fmt_ok!(
        "Service started at address {}",
        addr.color(OckamColor::PrimaryResource.color())
    ))?;

    if is_hop_service {
        opts.terminal.write_line(&fmt_warn!(
            "SECURITY WARNING: Don't use Hop service in production nodes"
        ))?;
    }

    Ok(())
}

/// Helper function.
pub(crate) async fn start_service_impl<T>(
    ctx: &Context,
    node: &BackgroundNodeClient,
    serv_name: &str,
    req: Request<T>,
) -> Result<()>
where
    T: Encode<()>,
{
    Ok(node
        .tell(ctx, req)
        .await
        .map_err(|e| miette!("Failed to start {} service: {e:?}", serv_name))?)
}

/// Public so `ockam_command::node::create` can use it.
pub async fn start_hop_service(
    ctx: &Context,
    node: &BackgroundNodeClient,
    serv_addr: &str,
) -> Result<()> {
    let req = api::start_hop_service(serv_addr);
    start_service_impl(ctx, node, "Hop", req).await
}
