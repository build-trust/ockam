use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::terminal::OckamColor;
use crate::util::{api, node_rpc, RpcBuilder};
use crate::{fmt_ok, CommandGlobalOpts};
use crate::{fmt_warn, Result};
use anyhow::anyhow;
use clap::{Args, Subcommand};

use colorful::Colorful;
use minicbor::Encode;
use ockam::{Context, TcpTransport};

use ockam_api::DefaultAddress;
use ockam_core::api::{RequestBuilder, Status};

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
    Identity {
        #[arg(long, default_value_t = identity_default_addr())]
        addr: String,
    },
    Authenticated {
        #[arg(long, default_value_t = authenticated_default_addr())]
        addr: String,
    },
    Verifier {
        #[arg(long, default_value_t = verifier_default_addr())]
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

fn identity_default_addr() -> String {
    DefaultAddress::IDENTITY_SERVICE.to_string()
}

fn authenticated_default_addr() -> String {
    DefaultAddress::AUTHENTICATED_SERVICE.to_string()
}

fn verifier_default_addr() -> String {
    DefaultAddress::VERIFIER.to_string()
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

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, StartCommand),
) -> crate::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: StartCommand,
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let tcp = TcpTransport::create(ctx).await?;
    let mut is_hop_service = false;
    let addr = match cmd.create_subcommand {
        StartSubCommand::Hop { addr, .. } => {
            is_hop_service = true;
            start_hop_service(ctx, &opts, &node_name, &addr, Some(&tcp)).await?;
            addr
        }
        StartSubCommand::Identity { addr, .. } => {
            start_identity_service(ctx, &opts, &node_name, &addr, Some(&tcp)).await?;
            addr
        }
        StartSubCommand::Authenticated { addr, .. } => {
            let req = api::start_authenticated_service(&addr);
            start_service_impl(ctx, &opts, &node_name, "Authenticated", req, Some(&tcp)).await?;
            addr
        }
        StartSubCommand::Verifier { addr, .. } => {
            start_verifier_service(ctx, &opts, &node_name, &addr, Some(&tcp)).await?;
            addr
        }
        StartSubCommand::Credentials {
            identity,
            addr,
            oneway,
            ..
        } => {
            let req = api::start_credentials_service(&identity, &addr, oneway);
            start_service_impl(ctx, &opts, &node_name, "Credentials", req, Some(&tcp)).await?;
            addr
        }
        StartSubCommand::Authenticator { addr, project, .. } => {
            start_authenticator_service(ctx, &opts, &node_name, &addr, &project, Some(&tcp))
                .await?;
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
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_name: &str,
    req: RequestBuilder<'_, T>,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()>
where
    T: Encode<()>,
{
    let mut rpc = RpcBuilder::new(ctx, opts, node_name).tcp(tcp)?.build();
    rpc.request(req).await?;

    let (res, _dec) = rpc.check_response()?;
    match res.status() {
        Some(Status::Ok) => Ok(()),
        _ => Err(anyhow!("Failed to start {serv_name} service").into()),
    }
}

/// Public so `ockam_command::node::create` can use it.
pub async fn start_hop_service(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_addr: &str,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()> {
    let req = api::start_hop_service(serv_addr);
    start_service_impl(ctx, opts, node_name, "Hop", req, tcp).await
}

/// Public so `ockam_command::node::create` can use it.
pub async fn start_identity_service(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_addr: &str,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()> {
    let req = api::start_identity_service(serv_addr);
    start_service_impl(ctx, opts, node_name, "Identity", req, tcp).await
}

/// Public so `ockam_command::node::create` can use it.
pub async fn start_verifier_service(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_addr: &str,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()> {
    let req = api::start_verifier_service(serv_addr);
    start_service_impl(ctx, opts, node_name, "Verifier", req, tcp).await
}

/// Public so `ockam_command::node::create` can use it.
#[allow(clippy::too_many_arguments)]
pub async fn start_authenticator_service(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_addr: &str,
    project: &str,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()> {
    let req = api::start_authenticator_service(serv_addr, project);
    start_service_impl(ctx, opts, node_name, "Authenticator", req, tcp).await
}
