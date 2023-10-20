use clap::Args;
use colorful::Colorful;

use ockam_api::cli_state::{NodeState, StateDirTrait, StateItemTrait};
use ockam_api::nodes::BackgroundNode;
use ockam_node::Context;

use crate::node::show::print_query_status;
use crate::node::util::spawn_node;
use crate::util::node_rpc;
use crate::{docs, fmt_err, fmt_info, fmt_log, fmt_ok, CommandGlobalOpts, OckamColor};

use super::util::check_default;

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
    (mut opts, cmd): (CommandGlobalOpts, StartCommand),
) -> miette::Result<()> {
    //If a node is given
    if let Some(node_name) = cmd.node_name {
        let node_state = opts.state.nodes.get(&node_name)?;

        opts.global_args.verbose = node_state.config().setup().verbose;

        // Abort if node is already running
        if node_state.is_running() {
            let n = node_state.name();
            opts.terminal
                .stdout()
                .plain(fmt_err!(
                    "The node '{n}' is already running. If you want to restart it you can call `ockam node stop {n}` and then `ockam node start {n}`"
                ))
                .write_line()?;
            return Ok(());
        }

        let mut node = run_node(&node_state, &ctx, &opts).await?;
        let is_default = check_default(&opts, &node_name);
        print_query_status(&opts, &ctx, &node_name, &mut node, true, is_default).await?;
        return Ok(());
    }

    //Terminal is not interactive or the quiet input or --no-input flag
    if !(opts.terminal.can_ask_for_user_input()) {
        let default_node = opts.state.nodes.default()?;
        run_node(&default_node, &ctx, &opts).await?;
        opts.terminal
            .stdout()
            .plain(fmt_ok!("{}", default_node.name()))
            .write_line()?;
        return Ok(());
    }

    //Get the inactive nodes list
    let node_list = opts.state.nodes.list()?;
    let inactive_nodes: Vec<String> = node_list
        .iter()
        .filter(|node_state| !(node_state.is_running()))
        .map(|node| node.name().to_owned())
        .collect();

    if inactive_nodes.is_empty() {
        opts.terminal
            .stdout()
            .plain(fmt_info!(
                "All the nodes are already started, nothing to do. Exiting gratefully"
            ))
            .write_line()?;
        return Ok(());
    }

    //Get the user nodes selection
    let user_selection = opts
        .terminal
        .select_multiple("Select the nodes".to_string(), inactive_nodes);

    //Exiting gratefully if no nodes are selected informing the user
    if user_selection.is_empty() {
        opts.terminal
            .stdout()
            .plain(fmt_info!("No node selected, exiting gratefully!"))
            .write_line()?;
        return Ok(());
    }

    //Ask for confirmation
    if !opts.terminal.confirm_interactively(format!(
        "You are about to start the given nodes: {}. Confirm?",
        &user_selection.join(", ")
    )) {
        opts.terminal
            .stdout()
            .plain(fmt_info!("No node selected, exiting gratefully!"))
            .write_line()?;
        return Ok(());
    }

    let user_selection: Vec<NodeState> = node_list
        .into_iter()
        .filter(|node| !user_selection.iter().all(|name| name != node.name()))
        .collect();

    let mut node_error_flag: bool = false;
    let mut node_starts_output: Vec<String> = vec![];
    for node_state in &user_selection {
        match run_node(node_state, &ctx, &opts).await {
            Ok(_) => node_starts_output.push(fmt_ok!("{}", &node_state.name().to_owned())),
            Err(_) => {
                node_error_flag = true;
                node_starts_output.push(format!(
                    "     ⚠️ {} wont start",
                    &node_state.name().to_owned()
                ))
            }
        }
    }

    if node_error_flag {
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

    opts.terminal
        .stdout()
        .plain(node_starts_output.join("\n"))
        .write_line()?;

    Ok(())
}

/// Run a single node. Return the BackgroundNode istance of the created node or error.
async fn run_node(
    node_state: &NodeState,
    ctx: &Context,
    opts: &CommandGlobalOpts,
) -> miette::Result<BackgroundNode> {
    node_state.kill_process(false)?;
    let node_setup = node_state.config().setup();
    let node_name = node_state.name();
    // Restart node
    spawn_node(
        opts,
        node_name,                                     // The selected node name
        &node_setup.api_transport()?.addr.to_string(), // The selected node api address
        None,                                          // No project information available
        None,                                          // No trusted identities
        None,                                          // "
        None,                                          // "
        None,                                          // Launch config
        None,                                          // Authority Identity
        None,                                          // Credential
        None,                                          // Trust Context
        None,                                          // Project Name
        true,                                          // Restarted nodes will log to files
    )?;

    let node = BackgroundNode::create(ctx, &opts.state, node_name).await?;
    Ok(node)
}
