use std::str::FromStr;

use clap::Args;
use colorful::Colorful;
use miette::Context as _;
use miette::{miette, IntoDiagnostic};
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::is_local_node;
use ockam_api::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;
use ockam_multiaddr::proto::Project;
use ockam_multiaddr::{MultiAddr, Protocol};

use crate::node::{get_node_name, initialize_node_if_default};
use crate::output::Output;
use crate::terminal::OckamColor;
use crate::util::{node_rpc, process_nodes_multiaddr};
use crate::{display_parse_logs, docs, fmt_ok, CommandGlobalOpts};
use crate::{fmt_log, Result};

const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create Relays
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Name of the relay
    #[arg(hide_default_value = true, default_value = "default")]
    relay_name: String,

    /// Node for which to create the relay
    #[arg(long, id = "NODE", display_order = 900)]
    to: Option<String>,

    /// Route to the node at which to create the relay
    #[arg(long, id = "ROUTE", display_order = 900, value_parser = parse_at, default_value_t = default_forwarder_at())]
    at: MultiAddr,

    /// Authorized identity for secure channel connection
    #[arg(long, id = "AUTHORIZED", display_order = 900)]
    authorized: Option<Identifier>,
}

impl CreateCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.to);
        node_rpc(rpc, (opts, self));
    }
}

fn parse_at(input: &str) -> Result<MultiAddr> {
    let mut at = input.to_string();
    if !input.contains('/') {
        at = format!("/node/{}", input);
    }

    let ma = MultiAddr::from_str(&at)?;

    Ok(ma)
}

pub fn default_forwarder_at() -> MultiAddr {
    MultiAddr::from_str("/project/default").expect("Default relay address is invalid")
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> miette::Result<()> {
    opts.terminal.write_line(&fmt_log!("Creating Relay...\n"))?;

    display_parse_logs(&opts);

    let to = get_node_name(&opts.state, &cmd.to);
    let node_name = extract_address_value(&to)?;
    let at_rust_node = is_local_node(&cmd.at).wrap_err("Argument --at is not valid")?;

    let ma = process_nodes_multiaddr(&cmd.at, &opts.state)?;
    let alias = if at_rust_node {
        format!("forward_to_{}", cmd.relay_name)
    } else {
        cmd.relay_name.clone()
    };

    let node = BackgroundNode::create(&ctx, &opts.state, &node_name).await?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let get_relay_info = async {
        let req = {
            let body = if cmd.at.matches(0, &[Project::CODE.into()]) {
                if cmd.authorized.is_some() {
                    return Err(
                        miette!("--authorized can not be used with project addresses").into(),
                    );
                }
                CreateForwarder::at_project(ma, Some(alias.clone()))
            } else {
                CreateForwarder::at_node(ma, Some(alias.clone()), at_rust_node, cmd.authorized)
            };
            Request::post("/node/forwarder").body(body)
        };

        let relay_info: ForwarderInfo = node.ask(&ctx, req).await?;
        *is_finished.lock().await = true;
        Ok(relay_info)
    };

    let output_messages = vec![
        format!(
            "Creating relay forwarding service at {}...",
            &cmd.at
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ),
        format!(
            "Setting up receiving relay mailbox on node {}...",
            &node_name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ),
    ];
    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (relay, _) = try_join!(get_relay_info, progress_output)?;

    let machine = relay.remote_address_ma().into_diagnostic()?;
    let json = serde_json::to_string_pretty(&relay).into_diagnostic()?;

    let formatted_from = format!(
        "{}{}",
        &cmd.at,
        &relay.worker_address_ma().into_diagnostic()?.to_string()
    )
    .color(OckamColor::PrimaryResource.color());
    let formatted_to = format!(
        "/node/{}{}",
        &node_name,
        &relay.remote_address_ma().into_diagnostic()?.to_string()
    )
    .color(OckamColor::PrimaryResource.color());

    opts.terminal
        .stdout()
        .plain(fmt_ok!(
            "Now relaying messages from {} → {}",
            formatted_from,
            formatted_to
        ))
        .machine(machine)
        .json(json)
        .write_line()?;

    Ok(())
}

impl Output for ForwarderInfo {
    fn output(&self) -> Result<String> {
        let output = format!(
            r#"
Relay {}:
    Route: {}
    Remote Address: {}
    Worker Address: {}
    Flow Control Id: {}"
"#,
            self.remote_address(),
            self.forwarding_route(),
            self.remote_address_ma()?,
            self.worker_address_ma()?,
            self.flow_control_id()
                .as_ref()
                .map(|x| x.to_string())
                .unwrap_or("<none>".into())
        );

        Ok(output)
    }

    fn list_output(&self) -> Result<String> {
        let output = format!(
            r#"Relay {}
Route {}"#,
            self.remote_address()
                .color(OckamColor::PrimaryResource.color()),
            self.forwarding_route()
                .color(OckamColor::PrimaryResource.color()),
        );

        Ok(output)
    }
}
