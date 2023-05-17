use std::str::FromStr;

use anyhow::{anyhow, Context as _};
use clap::Args;
use colorful::Colorful;
use ockam::identity::IdentityIdentifier;
use ockam_multiaddr::proto::Project;

use ockam::{Context, TcpTransport};
use ockam_api::is_local_node;
use ockam_api::nodes::models::forwarder::{CreateForwarder, ForwarderInfo};
use ockam_core::api::Request;
use ockam_multiaddr::{MultiAddr, Protocol};
use tokio::sync::Mutex;
use tokio::try_join;

use crate::node::get_node_name;
use crate::terminal::OckamColor;
use crate::util::output::Output;
use crate::util::{extract_address_value, node_rpc, process_nodes_multiaddr, RpcBuilder};
use crate::{docs, fmt_ok, CommandGlobalOpts};
use crate::{fmt_log, Result};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create Relays
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct CreateCommand {
    /// Name of the relay (optional)
    #[arg(hide_default_value = true, default_value = "default")]
    relay_name: String,

    /// Node for which to create the relay
    #[arg(long, id = "NODE", display_order = 900)]
    to: Option<String>,

    /// Route to the node at which to create the relay (optional)
    #[arg(long, id = "ROUTE", display_order = 900, value_parser = parse_at, default_value_t = default_forwarder_at())]
    at: MultiAddr,

    /// Authorized identity for secure channel connection (optional)
    #[arg(long, id = "AUTHORIZED", display_order = 900)]
    authorized: Option<IdentityIdentifier>,
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
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

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> Result<()> {
    opts.terminal.write_line(&fmt_log!("Creating Relay"))?;

    let tcp = TcpTransport::create(&ctx).await?;
    let to = get_node_name(&opts.state, cmd.to.clone())?;
    let api_node = extract_address_value(&to)?;
    let at_rust_node = is_local_node(&cmd.at).context("Argument --at is not valid")?;

    let ma = process_nodes_multiaddr(&cmd.at, &opts.state)?;
    let alias = if at_rust_node {
        format!("forward_to_{}", cmd.relay_name)
    } else {
        cmd.relay_name.clone()
    };

    let mut rpc = RpcBuilder::new(&ctx, &opts, &api_node).tcp(&tcp)?.build();
    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let req = {
            let body = if cmd.at.matches(0, &[Project::CODE.into()]) {
                if cmd.authorized.is_some() {
                    return Err(
                        anyhow!("--authorized can not be used with project addresses").into(),
                    );
                }
                CreateForwarder::at_project(ma, Some(alias.clone()))
            } else {
                CreateForwarder::at_node(ma, Some(alias.clone()), at_rust_node, cmd.authorized)
            };
            Request::post("/node/forwarder").body(body)
        };

        rpc.request(req).await?;

        *is_finished.lock().await = true;

        rpc.parse_response::<ForwarderInfo>()
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
            &api_node
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ),
    ];
    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (relay, _) = try_join!(send_req, progress_output)?;

    let machine = relay.remote_address_ma()?;
    let json = serde_json::to_string_pretty(&relay)?;

    let formatted_from = format!("{}{}", &cmd.at, &relay.worker_address_ma()?.to_string())
        .color(OckamColor::PrimaryResource.color());
    let formatted_to = format!(
        "/node/{}{}",
        &api_node,
        &relay.remote_address_ma()?.to_string()
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

impl Output for ForwarderInfo<'_> {
    fn output(&self) -> Result<String> {
        let output = format!(
            r#"
Relay {}:
    Forwarding Address: {} => {},
    Remote Address: {},
    Worker Address: {}
"#,
            self.remote_address(),
            self.worker_address_ma()?,
            self.remote_address_ma()?,
            self.remote_address_ma()?,
            self.worker_address_ma()?
        );

        Ok(output)
    }
}
