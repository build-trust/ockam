use clap::Args;
use colorful::Colorful;
use miette::miette;

use ockam::Context;
use ockam_api::nodes::models;
use ockam_core::api::Request;

use crate::node::{get_node_name, initialize_node_if_default};
use crate::terminal::ConfirmResult;
use crate::util::parse_node_name;
use crate::util::{node_rpc, Rpc};
use crate::{docs, node::NodeOpts, CommandGlobalOpts};

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP listener
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Tcp Listener ID
    pub address: String,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(run_impl, (opts, self));
    }
}

async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node = parse_node_name(&node_name)?;
    if cmd.yes {
        let mut rpc = Rpc::background(&ctx, &opts, &node)?;
        let req = Request::delete("/node/tcp/listener")
            .body(models::transport::DeleteTransport::new(cmd.address.clone()));
        rpc.request(req).await?;
        rpc.is_ok()?;
    } else {
        match opts
            .terminal
            .confirm("This will delete the selected Tcp-listener. Are you sure?")?
        {
            ConfirmResult::Yes => {}
            ConfirmResult::No => {
                return Ok(());
            }
            ConfirmResult::NonTTY => {
                return Err(miette!("Use --yes to confirm").into());
            }
        }
        let mut rpc = Rpc::background(&ctx, &opts, &node)?;
        let req = Request::delete("/node/tcp/listener")
            .body(models::transport::DeleteTransport::new(cmd.address.clone()));
        rpc.request(req).await?;
        rpc.is_ok()?;
    }

    // Print message
    print_req_resp(cmd.address.clone(), opts).await;
    Ok(())
}

/// Print the appropriate message after deletion.
async fn print_req_resp(node: String, opts: CommandGlobalOpts) {
    opts.terminal
        .stdout()
        .plain(format!(
            "{} TCP listener {node} has been successfully deleted.",
            "✔︎".light_green(),
        ))
        .json(serde_json::json!({ "tcp-listener": {"node": node } }))
        .write_line()
        .unwrap();
}
