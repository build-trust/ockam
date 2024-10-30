use clap::{Args, Subcommand};
use colorful::Colorful;
use miette::WrapErr;
use minicbor::Encode;

use crate::{CommandGlobalOpts, Result};
use ockam::Context;
use ockam_api::colors::OckamColor;
use ockam_api::nodes::service::default_address::DefaultAddress;
use ockam_api::nodes::BackgroundNodeClient;
use ockam_api::{fmt_ok, fmt_warn};
use ockam_core::api::Request;

use crate::node::NodeOpts;
use crate::util::{api, async_cmd};

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
    RemoteProxyVault {
        #[arg(long, default_value_t = remote_proxy_vault_default_addr())]
        addr: String,
        /// Name of the vault to expose
        #[arg(long)]
        vault_name: String,
    },
}

fn hop_default_addr() -> String {
    DefaultAddress::HOP_SERVICE.to_string()
}

fn remote_proxy_vault_default_addr() -> String {
    DefaultAddress::REMOTE_PROXY_VAULT.to_string()
}

impl StartCommand {
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "service start".into()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &self.node_opts.at_node).await?;
        let addr = match &self.create_subcommand {
            StartSubCommand::Hop { addr, .. } => {
                start_hop_service(ctx, &node, addr).await?;
                opts.terminal.write_line(fmt_warn!(
                    "SECURITY WARNING: Don't use Hop service in production nodes"
                ))?;
                addr
            }
            StartSubCommand::RemoteProxyVault { addr, vault_name } => {
                start_remote_proxy_vault_service(ctx, &node, addr, vault_name).await?;
                addr
            }
        };

        opts.terminal.write_line(fmt_ok!(
            "Service started at address {}",
            addr.to_string().color(OckamColor::PrimaryResource.color())
        ))?;

        Ok(())
    }
}

/// Helper function.
pub(crate) async fn start_service_impl<T>(
    ctx: &Context,
    node: &BackgroundNodeClient,
    service_name: &str,
    req: Request<T>,
) -> Result<()>
where
    T: Encode<()>,
{
    node.tell(ctx, req)
        .await
        .wrap_err(format!("Failed to start {service_name} service"))
}

/// Public so `ockam_command::node::create` can use it.
pub async fn start_hop_service(
    ctx: &Context,
    node: &BackgroundNodeClient,
    service_addr: &str,
) -> Result<()> {
    let req = api::start_hop_service(service_addr);
    start_service_impl(ctx, node, "Hop", req).await
}

/// Public so `ockam_command::node::create` can use it.
pub async fn start_remote_proxy_vault_service(
    ctx: &Context,
    node: &BackgroundNodeClient,
    service_addr: &str,
    vault_name: &str,
) -> Result<()> {
    let req = api::start_remote_proxy_vault_service(service_addr, vault_name);
    start_service_impl(ctx, node, "Remote Proxy Vault", req).await
}
