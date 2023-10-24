use clap::Args;
use colorful::Colorful;
use indoc::formatdoc;
use miette::{miette, IntoDiagnostic};
use tracing::trace;

use ockam::Context;
use ockam_api::address::extract_address_value;
use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::models::relay::RelayInfo;
use ockam_api::nodes::BackgroundNode;
use ockam_core::api::Request;
use ockam_multiaddr::MultiAddr;

use serde::Serialize;

use crate::node::{get_default_node_name, get_node_name};
use crate::output::Output;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts, OckamColor};

use tokio::sync::Mutex;
use tokio::try_join;

const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/show/after_long_help.txt");

/// Show a Relay by its alias
#[derive(Clone, Debug, Args)]
#[command(
    arg_required_else_help = false,
    before_help = docs::before_help(PREVIEW_TAG),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ShowCommand {
    /// Name assigned to relay that will be shown (prefixed with forward_to_<name>)
    #[arg(display_order = 900)]
    remote_address: Option<String>,
    //#[arg(display_order = 900, required = true)]
    //remote_address: String,
    /// Node which relay belongs to
    #[arg(display_order = 901, global = true, long, value_name = "NODE")]
    pub at: Option<String>,
}

impl ShowCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(run_impl, (options, self));
    }
}

#[derive(Serialize)]
struct RelayShowOutput {
    pub relay_route: String,
    pub remote_address: MultiAddr,
    pub worker_address: MultiAddr,
}

impl Output for RelayShowOutput {
    fn output(&self) -> crate::error::Result<String> {
        Ok(formatdoc!(
            r#"
        Relay:
            Relay Route: {route}
            Remote Address: {remote_addr}
            Worker Address: {worker_addr}
        "#,
            route = self.relay_route,
            remote_addr = self.remote_address,
            worker_addr = self.worker_address,
        ))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ShowCommand),
) -> miette::Result<()> {
    // The remote address is provided, no need to prompt the user
    if cmd.remote_address.is_some() {
        let remote_addr = cmd.remote_address.as_ref().unwrap();
        let at = get_node_name(&opts.state, &cmd.at);
        let node_name = extract_address_value(&at)?;
        show_single_relay(&ctx, &opts, remote_addr, &node_name).await?;
        return Ok(());
    }

    // from this point, the code operates on the default node.
    // and the --at option is disregarded.
    let default_node = get_default_node_name(&opts.state);
    let default_node_name = extract_address_value(&default_node)?;

    // The user has not provided any remote address.
    // Before checking interactivity, check there are indeed
    // some relays on the default node.
    let relays = list_relays(&ctx, &opts, &default_node_name).await?;
    match relays.len() {
        0 => {
            opts.terminal
                .stdout()
                .plain("The default node does not have any relays")
                .write_line()?;
            return Ok(());
        }

        1 => {
            // Only one relay on the default node. Show that.
            show_single_relay(&ctx, &opts, relays[0].remote_address(), &default_node_name).await?;
            return Ok(());
        }
        _ => {
            if opts.terminal.can_ask_for_user_input() {
                // The remote address was not provided, but there are
                // more than one relay on the default node
                // and the user can be prompted.
                show_relay_list(relays, &opts, &default_node_name).await?;
                return Ok(());
            }
        }
    }

    // If we get here, then very little can be done.
    opts.terminal
        .stdout()
        .plain("The remote address must be specified")
        .write_line()?;

    Ok(())
}

async fn show_single_relay(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    remote_address: &str,
    node_name: &str,
) -> miette::Result<()> {
    let node = BackgroundNode::create(&ctx, &opts.state, node_name).await?;
    let relay_info: RelayInfo = node
        .ask(
            &ctx,
            Request::get(format!("/node/forwarder/{remote_address}")),
        )
        .await?;

    let output = RelayShowOutput {
        relay_route: relay_info.forwarding_route().to_string(),
        remote_address: relay_info.remote_address_ma().into_diagnostic()?,
        worker_address: relay_info.worker_address_ma().into_diagnostic()?,
    };

    opts.terminal
        .clone()
        .stdout()
        .plain(output.output()?)
        .json(serde_json::to_string(&output).into_diagnostic()?)
        .write_line()?;

    Ok(())
}

async fn show_relay_list(
    relays: Vec<RelayInfo>,
    opts: &CommandGlobalOpts,
    node_name: &str,
) -> miette::Result<()> {
    let remote_addresses = relays
        .iter()
        .map(|it| it.remote_address().to_string())
        .collect::<Vec<String>>();

    let selected_remote_addresses = opts.terminal.select_multiple(
        "Select one or more relays that you want to show".to_string(),
        remote_addresses,
    );

    if selected_remote_addresses.is_empty() {
        opts.terminal
            .clone()
            .stdout()
            .plain("No remote address selected")
            .write_line()?;
        return Ok(());
    }

    // reduce the list of relays to the one(s) that were selected.
    let mut selected_relays = Vec::<RelayInfo>::new();
    for a in selected_remote_addresses {
        for r in &relays {
            if a == r.remote_address() {
                selected_relays.push(r.clone());
                break;
            }
        }
    }

    let plain = opts.terminal.build_list(
        &selected_relays,
        &format!("Relays on Node {node_name}"),
        &format!("No Relays found on node {node_name}."),
    )?;
    let json = serde_json::to_string_pretty(&selected_relays).into_diagnostic()?;

    opts.terminal
        .clone()
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;
    return Ok(());
}

async fn list_relays(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_name: &str,
) -> miette::Result<Vec<RelayInfo>> {
    if !opts.state.nodes.get(node_name)?.is_running() {
        return Err(miette!("The node '{}' is not running", node_name));
    }

    let node = BackgroundNode::create(&ctx, &opts.state, node_name).await?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let get_relays = async {
        let relay_infos: Vec<RelayInfo> = node.ask(&ctx, Request::get("/node/forwarder")).await?;
        *is_finished.lock().await = true;
        Ok(relay_infos)
    };

    let output_messages = vec![format!(
        "Listing Relays on {}...\n",
        node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (relays, _) = try_join!(get_relays, progress_output)?;
    trace!(?relays, "Relays retrieved");

    return Ok(relays);
}
