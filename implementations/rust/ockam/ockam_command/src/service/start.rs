use clap::{Args, Subcommand};
use colorful::Colorful;
use miette::miette;
use minicbor::Encode;

use ockam::Context;
use ockam_api::DefaultAddress;
use ockam_core::api::Request;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::terminal::OckamColor;
use crate::util::{api, node_rpc, Rpc};
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
    Authenticated {
        #[arg(long, default_value_t = authenticated_default_addr())]
        addr: String,
    },
    Credentials {
        #[arg(long)]
        identity: String,

        #[arg(long, default_value_t = credentials_default_addr())]
        addr: String,

        #[arg(long)]
        oneway: bool,
    },
    Authenticator {
        #[arg(long, default_value_t = authenticator_default_addr())]
        addr: String,

        #[arg(long)]
        project: String,
    },
}

fn hop_default_addr() -> String {
    DefaultAddress::HOP_SERVICE.to_string()
}

fn authenticated_default_addr() -> String {
    DefaultAddress::AUTHENTICATED_SERVICE.to_string()
}

fn credentials_default_addr() -> String {
    DefaultAddress::CREDENTIALS_SERVICE.to_string()
}

fn authenticator_default_addr() -> String {
    DefaultAddress::DIRECT_AUTHENTICATOR.to_string()
}

impl StartCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, StartCommand)) -> miette::Result<()> {
    run_impl(ctx, opts, cmd).await
}

async fn run_impl(ctx: Context, opts: CommandGlobalOpts, cmd: StartCommand) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let mut rpc = Rpc::background(&ctx, &opts, &node_name).await?;
    let mut is_hop_service = false;
    let addr = match cmd.create_subcommand {
        StartSubCommand::Hop { addr, .. } => {
            is_hop_service = true;
            start_hop_service(&mut rpc, &addr).await?;
            addr
        }
        StartSubCommand::Authenticated { addr, .. } => {
            let req = api::start_authenticated_service(&addr);
            start_service_impl(&mut rpc, "Authenticated", req).await?;
            addr
        }
        StartSubCommand::Credentials {
            identity,
            addr,
            oneway,
            ..
        } => {
            let req = api::start_credentials_service(&identity, &addr, oneway);
            start_service_impl(&mut rpc, "Credentials", req).await?;
            addr
        }
        StartSubCommand::Authenticator { addr, project, .. } => {
            start_authenticator_service(&mut rpc, &addr, &project).await?;
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
pub(crate) async fn start_service_impl<'a, T>(
    rpc: &mut Rpc,
    serv_name: &str,
    req: Request<T>,
) -> Result<()>
where
    T: Encode<()>,
{
    rpc.tell(req)
        .await
        .map_err(|_| miette!("Failed to start {} service", serv_name).into())
}

/// Public so `ockam_command::node::create` can use it.
pub async fn start_hop_service<'a>(rpc: &mut Rpc, serv_addr: &str) -> Result<()> {
    let req = api::start_hop_service(serv_addr);
    start_service_impl(rpc, "Hop", req).await
}

/// Public so `ockam_command::node::create` can use it.
#[allow(clippy::too_many_arguments)]
pub async fn start_authenticator_service<'a>(
    rpc: &mut Rpc,
    serv_addr: &str,
    project: &str,
) -> Result<()> {
    let req = api::start_authenticator_service(serv_addr, project);
    start_service_impl(rpc, "Authenticator", req).await
}
