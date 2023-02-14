use crate::util::{
    bind_to_port_check, exitcode, extract_address_value, node_rpc, process_nodes_multiaddr,
    RpcBuilder,
};
use crate::Result;
use crate::{help, CommandGlobalOpts};
use anyhow::anyhow;
use clap::Args;
use ockam::identity::IdentityIdentifier;
use ockam::{Context, TcpTransport};
use ockam_api::nodes::models::portal::CreateInlet;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_core::api::Request;
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol as _};
use std::net::SocketAddr;

const HELP_DETAIL: &str = include_str!("../../constants/tcp/inlet/help_detail.txt");

/// Create TCP Inlets
#[derive(Clone, Debug, Args)]
#[command(after_long_help = help::template(HELP_DETAIL))]
pub struct CreateCommand {
    /// Node on which to start the tcp inlet.
    #[arg(long, display_order = 900, id = "NODE")]
    at: String,

    /// Address on which to accept tcp connections.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS")]
    from: SocketAddr,

    /// Route to a tcp outlet.
    #[arg(long, display_order = 900, id = "ROUTE")]
    to: MultiAddr,

    /// Authorized identity for secure channel connection
    #[arg(long, name = "AUTHORIZED", display_order = 900)]
    authorized: Option<IdentityIdentifier>,

    /// Enable credential authorization.
    /// Defaults to the Node's `enable-credential-checks` value passed upon creation.
    #[arg(long, display_order = 900, conflicts_with = "disable_check_credential")]
    check_credential: bool,

    /// Disable credential authorization.
    /// Defaults to the Node's `enable-credential-checks` value passed upon creation.
    #[arg(long, display_order = 900, conflicts_with = "check_credential")]
    disable_check_credential: bool,

    /// Assign a name to this inlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }

    pub fn check_credential(&self) -> Option<bool> {
        if self.check_credential {
            Some(true)
        } else if self.disable_check_credential {
            Some(false)
        } else {
            None
        }
    }
}

async fn rpc(ctx: Context, (opts, mut cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    cmd.to = process_nodes_multiaddr(&cmd.to, &opts.state)?;

    // Check if the port is used by some other services or process
    if !bind_to_port_check(&cmd.from) {
        return Err(crate::error::Error::new(
            exitcode::IOERR,
            anyhow!("Another process is listening on the provided port!"),
        ));
    }

    let tcp = TcpTransport::create(&ctx).await?;
    let node = extract_address_value(&cmd.at)?;

    let req = {
        let check_credential = cmd.check_credential();
        let mut payload = if cmd.to.matches(0, &[Project::CODE.into()]) {
            if cmd.authorized.is_some() {
                return Err(anyhow!("--authorized can not be used with project addresses").into());
            }
            CreateInlet::via_project(cmd.from, cmd.to, check_credential)
        } else {
            CreateInlet::to_node(cmd.from, cmd.to, check_credential, cmd.authorized)
        };
        if let Some(a) = cmd.alias {
            payload.set_alias(a)
        }
        Request::post("/node/inlet").body(payload)
    };

    let mut rpc = RpcBuilder::new(&ctx, &opts, &node).tcp(&tcp)?.build();
    rpc.request(req).await?;
    rpc.parse_response::<InletStatus>()?;

    Ok(())
}

fn alias_parser(arg: &str) -> Result<String> {
    if arg.contains(':') {
        Err(anyhow!("an inlet alias must not contain ':' characters").into())
    } else {
        Ok(arg.to_string())
    }
}
