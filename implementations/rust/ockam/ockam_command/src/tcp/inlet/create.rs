use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::thread::sleep;
use std::time::Duration;

use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::log::trace;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_abac::Resource;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::nodes::models::portal::CreateInlet;
use ockam_api::nodes::models::portal::InletStatus;
use ockam_core::api::{Reply, Request, Status};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{route, Error};
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol as _};

use crate::node::{get_node_name, initialize_node_if_default};
use crate::policy::{add_default_project_policy, has_policy};
use crate::tcp::util::alias_parser;
use crate::terminal::OckamColor;
use crate::util::duration::duration_parser;
use crate::util::parsers::socket_addr_parser;
use crate::util::{
    find_available_port, node_rpc, parse_node_name, port_is_free_guard, process_nodes_multiaddr,
    Rpc,
};
use crate::{display_parse_logs, docs, fmt_log, fmt_ok, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create TCP Inlets
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct CreateCommand {
    /// Node on which to start the tcp inlet.
    #[arg(long, display_order = 900, id = "NODE")]
    at: Option<String>,

    /// Address on which to accept tcp connections.
    #[arg(long, display_order = 900, id = "SOCKET_ADDRESS", hide_default_value = true, default_value_t = default_from_addr(), value_parser = socket_addr_parser)]
    from: SocketAddr,

    /// Route to a tcp outlet.
    #[arg(long, display_order = 900, id = "ROUTE", default_value_t = default_to_addr())]
    to: MultiAddr,

    /// Authorized identity for secure channel connection
    #[arg(long, name = "AUTHORIZED", display_order = 900)]
    authorized: Option<Identifier>,

    /// Assign a name to this inlet.
    #[arg(long, display_order = 900, id = "ALIAS", value_parser = alias_parser)]
    alias: Option<String>,

    /// Time to wait for the outlet to be available.
    #[arg(long, display_order = 900, id = "WAIT", default_value = "5s", value_parser = duration_parser)]
    connection_wait: Duration,

    /// Time to wait before retrying to connect to outlet.
    #[arg(long, display_order = 900, id = "RETRY", default_value = "20s", value_parser = duration_parser)]
    retry_wait: Duration,
}

pub(crate) fn default_from_addr() -> SocketAddr {
    let port = find_available_port().expect("Failed to find available port");
    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port)
}

fn default_to_addr() -> MultiAddr {
    MultiAddr::from_str("/project/default/service/forward_to_default/secure/api/service/outlet")
        .expect("Failed to parse default multiaddr")
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.at);
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(
    ctx: Context,
    (opts, mut cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    opts.terminal.write_line(&fmt_log!(
        "Creating TCP Inlet at {}...\n",
        cmd.from
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    ))?;
    display_parse_logs(&opts);

    cmd.to = process_nodes_multiaddr(&cmd.to, &opts.state)?;

    let node_name = get_node_name(&opts.state, &cmd.at);
    let node = parse_node_name(&node_name)?;

    let mut rpc = Rpc::background(&ctx, &opts.state, &node).await?;
    let is_finished: Mutex<bool> = Mutex::new(false);
    let progress_bar = opts.terminal.progress_spinner();
    let create_inlet = async {
        port_is_free_guard(&cmd.from)?;

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
                return Err(miette!("--authorized can not be used with project addresses").into());
            }
            true
        } else {
            false
        };

        let inlet = loop {
            let req = {
                let mut payload = if via_project {
                    CreateInlet::via_project(
                        cmd.from.to_string(),
                        cmd.to.clone(),
                        route![],
                        route![],
                    )
                } else {
                    CreateInlet::to_node(
                        cmd.from.to_string(),
                        cmd.to.clone(),
                        route![],
                        route![],
                        cmd.authorized.clone(),
                    )
                };
                if let Some(a) = cmd.alias.as_ref() {
                    payload.set_alias(a)
                }
                payload.set_wait_ms(cmd.connection_wait.as_millis() as u64);

                Request::post("/node/inlet").body(payload)
            };

            let result: Reply<InletStatus> = rpc.ask_and_get_reply(req).await?;

            match result {
                Reply::Successful(inlet_status) => {
                    *is_finished.lock().await = true;
                    break inlet_status;
                }
                Reply::Failed(e, s) => {
                    if let Some(status) = s {
                        if status == Status::BadRequest {
                            Err(Error::new(
                                Origin::Api,
                                Kind::Invalid,
                                e.message().unwrap_or("bad request when creating an inlet"),
                            ))?
                        }
                    };
                    trace!("the inlet creation returned a non-OK status: {s:?}");

                    if cmd.retry_wait.as_millis() == 0 {
                        return Err(miette!("Failed to create TCP inlet"))?;
                    }

                    if let Some(spinner) = progress_bar.as_ref() {
                        spinner.set_message(format!(
                            "Waiting for inlet {} to be available... Retrying momentarily",
                            &cmd.to
                                .to_string()
                                .color(OckamColor::PrimaryResource.color())
                        ));
                    }
                    sleep(cmd.retry_wait)
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

    let (inlet, _) = try_join!(create_inlet, progress_output)?;

    let machine_output = inlet.bind_addr.to_string();

    let json_output = serde_json::to_string_pretty(&inlet).into_diagnostic()?;

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
