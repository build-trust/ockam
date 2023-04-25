use crate::node::NodeOpts;
use crate::util::{api, node_rpc, RpcBuilder};
use crate::Result;
use crate::{fmt_warn, CommandGlobalOpts};
use anyhow::anyhow;
use clap::{Args, Subcommand};

use minicbor::Encode;
use ockam::{Context, TcpTransport};

use ockam_api::DefaultAddress;
use ockam_core::api::{RequestBuilder, Status};

/// Start a specified service
#[derive(Clone, Debug, Args)]
pub struct StartCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,

    #[command(subcommand)]
    pub create_subcommand: StartSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum StartSubCommand {
    Vault {
        #[arg(default_value_t = vault_default_addr())]
        addr: String,
    },
    Identity {
        #[arg(default_value_t = identity_default_addr())]
        addr: String,
    },
    Authenticated {
        #[arg(default_value_t = authenticated_default_addr())]
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

fn vault_default_addr() -> String {
    DefaultAddress::VAULT_SERVICE.to_string()
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
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
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
    let node_name = &cmd.node_opts.api_node;
    let tcp = TcpTransport::create(ctx).await?;
    match cmd.create_subcommand {
        StartSubCommand::Vault { addr, .. } => {
            start_vault_service(ctx, &opts, node_name, &addr, Some(&tcp)).await?
        }
        StartSubCommand::Identity { addr, .. } => {
            start_identity_service(ctx, &opts, node_name, &addr, Some(&tcp)).await?
        }
        StartSubCommand::Authenticated { addr, .. } => {
            let req = api::start_authenticated_service(&addr);
            start_service_impl(
                ctx,
                &opts,
                node_name,
                &addr,
                "Authenticated",
                req,
                Some(&tcp),
            )
            .await?
        }
        StartSubCommand::Verifier { addr, .. } => {
            start_verifier_service(ctx, &opts, node_name, &addr, Some(&tcp)).await?
        }
        StartSubCommand::Credentials {
            identity,
            addr,
            oneway,
            ..
        } => {
            let req = api::start_credentials_service(&identity, &addr, oneway);
            start_service_impl(ctx, &opts, node_name, &addr, "Credentials", req, Some(&tcp)).await?
        }
        StartSubCommand::Authenticator { addr, project, .. } => {
            start_authenticator_service(ctx, &opts, node_name, &addr, &project, Some(&tcp)).await?
        }
    }

    Ok(())
}

/// Helper function.
pub(crate) async fn start_service_impl<T>(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_addr: &str,
    serv_name: &str,
    req: RequestBuilder<'_, T>,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()>
where
    T: Encode<()>,
{
    let mut rpc = RpcBuilder::new(ctx, opts, node_name).tcp(tcp)?.build();
    rpc.request(req).await?;

    let (res, dec) = rpc.check_response()?;
    match res.status() {
        Some(Status::Ok) => {
            println!("{serv_name} service started at address: {serv_addr}");
            Ok(())
        }
        _ => {
            eprintln!("{}", rpc.parse_err_msg(res, dec));
            Err(anyhow!("Failed to start {serv_name} service").into())
        }
    }
}

/// Public so `ockam_command::node::create` can use it.
pub async fn start_vault_service(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
    serv_addr: &str,
    tcp: Option<&'_ TcpTransport>,
) -> Result<()> {
    let req = api::start_vault_service(serv_addr);
    start_service_impl(ctx, opts, node_name, serv_addr, "Vault", req, tcp).await
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
    start_service_impl(ctx, opts, node_name, serv_addr, "Identity", req, tcp).await
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
    start_service_impl(ctx, opts, node_name, serv_addr, "Verifier", req, tcp).await
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
    start_service_impl(ctx, opts, node_name, serv_addr, "Authenticator", req, tcp).await
}
