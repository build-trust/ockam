use crate::node::get_node_name;
use crate::policy::{add_default_project_policy, has_policy};
use crate::tcp::util::alias_parser;
use crate::terminal::OckamColor;
use crate::util::parsers::socket_addr_parser;
use crate::util::{
    bind_to_port_check, exitcode, extract_address_value, find_available_port, node_rpc,
    process_nodes_multiaddr, RpcBuilder,
};
use crate::{fmt_log, fmt_ok, CommandGlobalOpts, Result};
use anyhow::anyhow;
use clap::Args;
use colorful::Colorful;
use ockam::identity::IdentityIdentifier;
use ockam::{Context, TcpTransport};
use ockam_abac::Resource;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::nodes::models::portal::CreateInlet;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_core::api::Request;
use ockam_core::route;
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol as _};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::try_join;

/// Create TCP Inlets
#[derive(Clone, Debug, Args)]
pub struct CreateCommand {
    /// Node on which to start the tcp inlet.
    #[arg(long, display_order = 900, id = "NODE")]
    at: Option<String>,

    /// Address on which to accept tcp connections.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", default_value_t = default_from_addr(), value_parser = socket_addr_parser)]
    from: SocketAddr,

    /// Route to a tcp outlet.
    #[arg(long, display_order = 900, id = "ROUTE", default_value_t = default_to_addr())]
    to: MultiAddr,

    /// Authorized identity for secure channel connection
    #[arg(long, name = "AUTHORIZED", display_order = 900)]
    authorized: Option<IdentityIdentifier>,

    /// Assign a name to this inlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,

    /// Time to wait for the outlet to be available (ms).
    #[arg(long, display_order = 900, id = "WAIT", default_value = "5000")]
    connection_wait_ms: u64,

    /// Time to wait before retrying to connect to outlet (ms).
    #[arg(long, display_order = 900, id = "RETRY", default_value = "20000")]
    retry_wait_ms: u64,
}

fn default_from_addr() -> SocketAddr {
    let port = find_available_port().expect("Failed to find available port");
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
}

fn default_to_addr() -> MultiAddr {
    MultiAddr::from_str("/project/default/service/forward_to_default/secure/api/service/outlet")
        .expect("Failed to parse default multiaddr")
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, mut cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    opts.terminal.write_line(&fmt_log!("Creating TCP Inlet"))?;
    cmd.to = process_nodes_multiaddr(&cmd.to, &opts.state)?;

    let node_name = get_node_name(&opts.state, cmd.at.clone())?;
    let node = extract_address_value(&node_name)?;

    let tcp = TcpTransport::create(&ctx).await?;
    let mut rpc = RpcBuilder::new(&ctx, &opts, &node).tcp(&tcp)?.build();
    let is_finished: Mutex<bool> = Mutex::new(false);
    let progress_bar = opts.terminal.progress_spinner();
    let send_req = async {
        // Check if the port is used by some other services or process
        if !bind_to_port_check(&cmd.from) {
            return Err(crate::error::Error::new(
                exitcode::IOERR,
                anyhow!("Another process is listening on the provided port!"),
            ));
        }

        let project = opts
            .state
            .nodes
            .get(&node)?
            .config()
            .setup()
            .project
            .to_owned();
        let resource = Resource::new("tcp-inlet");
        if let Some(p) = project {
            if !has_policy(&node, &ctx, &opts, &resource).await? {
                add_default_project_policy(&node, &ctx, &opts, p, &resource).await?;
            }
        }

        let via_project = if cmd.to.clone().matches(0, &[Project::CODE.into()]) {
            if cmd.authorized.is_some() {
                return Err(anyhow!("--authorized can not be used with project addresses").into());
            }
            true
        } else {
            false
        };

        let inlet = loop {
            let req = {
                let mut payload = if via_project {
                    CreateInlet::via_project(cmd.from, cmd.to.clone(), route![], route![])
                } else {
                    CreateInlet::to_node(
                        cmd.from,
                        cmd.to.clone(),
                        route![],
                        route![],
                        cmd.authorized.clone(),
                    )
                };
                if let Some(a) = cmd.alias.as_ref() {
                    payload.set_alias(a)
                }
                payload.set_wait_ms(cmd.connection_wait_ms);

                Request::post("/node/inlet").body(payload)
            };

            rpc.request(req).await?;

            match rpc.is_ok() {
                Ok(_) => {
                    *is_finished.lock().await = true;
                    break rpc.parse_response::<InletStatus>()?;
                }
                Err(_) => {
                    if let Some(spinner) = progress_bar.as_ref() {
                        spinner.set_message(format!(
                            "Waiting for outlet {} to be available... Retrying momentarily",
                            &cmd.to
                                .to_string()
                                .color(OckamColor::PrimaryResource.color())
                        ));
                    }
                    sleep(Duration::from_millis(cmd.retry_wait_ms))
                }
            }
        };

        Ok(inlet)
    };

    let progress_messages = vec![
        format!(
            "Creating TCP Inlet on {}...",
            &node.to_string().color(OckamColor::PrimaryResource.color())
        ),
        format!(
            "Hosting TCP Socket at {}...",
            &cmd.from
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ),
        format!(
            "Establishing connection to outlet {}...",
            &cmd.to
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ),
    ];
    let progress_output = opts.terminal.progress_output_with_progress_bar(
        &progress_messages,
        &is_finished,
        progress_bar.as_ref(),
    );

    let (inlet, _) = try_join!(send_req, progress_output)?;

    let machine_output = inlet.bind_addr.to_string();

    let json_output = serde_json::to_string_pretty(&inlet)?;

    opts.terminal
        .stdout()
        .plain(
            fmt_ok!(
                "TCP Inlet {} on node {} is now sending traffic\n",
                &cmd.from
                    .to_string()
                    .color(OckamColor::PrimaryResource.color()),
                &node.to_string().color(OckamColor::PrimaryResource.color())
            ) + &fmt_log!(
                "to the outlet at {}",
                &cmd.to
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ),
        )
        .machine(machine_output)
        .json(json_output)
        .write_line()?;

    Ok(())
}
