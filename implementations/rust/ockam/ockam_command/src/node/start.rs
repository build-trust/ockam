use clap::Args;
use colorful::Colorful;

use ockam_api::nodes::BackgroundNode;
use ockam_node::Context;

use crate::node::show::print_query_status;
use crate::node::util::spawn_node;
use crate::util::node_rpc;
use crate::{docs, fmt_err, fmt_info, fmt_log, fmt_ok, fmt_warn, CommandGlobalOpts, OckamColor};

const LONG_ABOUT: &str = include_str!("./static/start/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/start/after_long_help.txt");

/// Start a node that was previously stopped
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
before_help = docs::before_help(PREVIEW_TAG),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct StartCommand {
    /// Name of the node to be started
    node_name: Option<String>,

    #[arg(long, default_value = "false")]
    aws_kms: bool,
}

impl StartCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(run_impl, (opts, self))
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, StartCommand),
) -> miette::Result<()> {
    if cmd.node_name.is_some() || !opts.terminal.can_ask_for_user_input() {
        let node_name = opts.state.get_node_or_default(&cmd.node_name).await?.name();
        start_single_node(&node_name, opts, &ctx).await?;
        return Ok(());
    }

    let inactive_nodes = get_inactive_nodes(&opts).await?;
    match inactive_nodes.len() {
        0 => {
            opts.terminal
                .stdout()
                .plain(fmt_info!(
                    "All the nodes are already started, nothing to do. Exiting gratefully"
                ))
                .write_line()?;
        }
        1 => {
            start_single_node(&inactive_nodes[0], opts, &ctx).await?;
        }
        _ => {
            let selected_nodes = opts
                .terminal
                .select_multiple("Select the nodes".to_string(), inactive_nodes);
            match selected_nodes.len() {
                0 => {
                    opts.terminal
                        .stdout()
                        .plain(fmt_info!("No node selected, exiting gratefully!"))
                        .write_line()?;
                }
                1 => start_single_node(&selected_nodes[0], opts, &ctx).await?,
                _ => {
                    if !opts.terminal.confirm_interactively(format!(
                        "You are about to start the given nodes:[ {} ]. Confirm?",
                        &selected_nodes.join(", ")
                    )) {
                        opts.terminal
                            .stdout()
                            .plain(fmt_info!("No node selected, exiting gratefully!"))
                            .write_line()?;
                        return Ok(());
                    }

                    let formatted_starts_result =
                        start_multiple_nodes(&ctx, &opts, &selected_nodes).await?;

                    opts.terminal
                        .stdout()
                        .plain(formatted_starts_result.join("\n"))
                        .write_line()?;
                }
            }
        }
    }
    Ok(())
}

/// Starts a single node and display the output on the console
async fn start_single_node(
    node_name: &str,
    mut opts: CommandGlobalOpts,
    ctx: &Context,
) -> miette::Result<()> {
    let node_info = opts.state.get_node(node_name).await?;

    opts.global_args.verbose = node_info.verbosity();

    // Abort if node is already running
    if node_info.is_running() {
        opts.terminal
            .stdout()
            .plain(fmt_err!(
                "The node '{node_name}' is already running. If you want to restart it you can \
                    call `ockam node stop {node_name}` and then `ockam node start {node_name}`"
            ))
            .write_line()?;
        return Ok(());
    }

    let mut node: BackgroundNode = run_node(node_name, ctx, &opts).await?;
    print_query_status(&opts, ctx, &mut node, true).await?;
    Ok(())
}

/// Start multiples nodes and return a formatted result in the form as a list.
/// Eventually append info on how to find error logs if there are.
async fn start_multiple_nodes(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    node_selected: &[String],
) -> miette::Result<Vec<String>> {
    let mut node_error_flag: bool = false;
    let mut node_starts_output: Vec<String> = vec![];
    for node_name in node_selected {
        match run_node(node_name, ctx, opts).await {
            Ok(_) => node_starts_output.push(fmt_ok!("{node_name}")),
            Err(_) => {
                node_error_flag = true;
                node_starts_output.push(fmt_warn!("{node_name}"))
            }
        }
    }
    if node_error_flag {
        append_info_if_errors(&mut node_starts_output);
    };
    Ok(node_starts_output)
}

/// Run a single node. Return the BackgroundNode instance of the created node or error
async fn run_node(
    node_name: &str,
    ctx: &Context,
    opts: &CommandGlobalOpts,
) -> miette::Result<BackgroundNode> {
    let node_info = opts.state.get_node(node_name).await?;
    opts.state.stop_node(node_name, false).await?;
    let node_address = node_info
        .tcp_listener_address()
        .map(|a| a.to_string())
        .unwrap_or("no transport address".to_string());

    // Restart node
    spawn_node(
        opts,
        node_name,     // The selected node name
        &None,         // Use the default identity
        &node_address, // The selected node api address
        None,          // No project information available
        None,          // No trusted identities
        None,          // "
        None,          // Launch config
        None,          // Authority Identity
        None,          // Credential
        None,          // Trust Context
        true,          // Restarted nodes will log to files
    )
    .await?;

    let node = BackgroundNode::create_to_node(ctx, &opts.state, node_name).await?;
    Ok(node)
}

/// Get a list of the inactive_nodes
async fn get_inactive_nodes(opts: &CommandGlobalOpts) -> miette::Result<Vec<String>> {
    let node_list = opts.state.get_nodes().await?;
    Ok(node_list
        .iter()
        .filter(|node_state| !(node_state.is_running()))
        .map(|node| node.name().to_owned())
        .collect())
}

/// Append the information on how to retrieve error to the result list in case of errors
fn append_info_if_errors(node_starts_output: &mut Vec<String>) {
    node_starts_output.push(
        "\n\n".to_string()
            + &fmt_err!("You can check the status of failed nodes using the command\n")
            + &fmt_log!(
                "{}",
                "ockam node show\n".color(OckamColor::PrimaryResource.color())
            )
            + &fmt_log!("or check the logs with the command\n")
            + &fmt_log!(
                "{}",
                "ockam node logs".color(OckamColor::PrimaryResource.color())
            ),
    );
}
