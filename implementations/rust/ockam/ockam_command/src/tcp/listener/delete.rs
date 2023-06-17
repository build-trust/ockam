use miette::miette;
use clap::Args;
use colorful::Colorful;

use crate::util::parse_node_name;
use ockam::Context;
use ockam_api::nodes::models;
use ockam_core::api::Request;

use crate::node::{get_node_name, initialize_node_if_default};
use crate::util::{node_rpc, Rpc};
use crate::{docs, node::NodeOpts, CommandGlobalOpts};
use crate::terminal::ConfirmResult;

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
) -> crate::Result<()> {
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node = parse_node_name(&node_name)?;
    if cmd.yes {
        let mut rpc = Rpc::background(&ctx, &opts, &node)?;
        let req = Request::delete("/node/tcp/listener")
            .body(models::transport::DeleteTransport::new(cmd.address.clone()));
        rpc.request(req).await?;
        rpc.is_ok()?;

        opts.terminal
            .stdout()
            .plain(format!(
                "{} TCP listener with address '{}' has been deleted.",
                "✔︎".light_green(),
                &cmd.address
            ))
            .machine(&cmd.address)
            .json(serde_json::json!({ "TCP listener": { "address": &cmd.address } }))
            .write_line()?;
    } else {
        match opts.terminal.confirm("This will delete the selected Tcp-listener. Are you sure?")? {
            ConfirmResult::Yes => {
                let mut rpc = Rpc::background(&ctx, &opts, &node)?;
                let req = Request::delete("/node/tcp/listener")
                    .body(models::transport::DeleteTransport::new(cmd.address.clone()));
                rpc.request(req).await?;
                rpc.is_ok()?;

                opts.terminal
                    .stdout()
                    .plain(format!(
                        "{} TCP listener with address '{}' has been deleted.",
                        "✔︎".light_green(),
                        &cmd.address
                    ))
                    .machine(&cmd.address)
                    .json(serde_json::json!({ "TCP listener": { "address": &cmd.address } }))
                    .write_line()?;
            }
            ConfirmResult::No => {
                return Ok(());
            }
            ConfirmResult::NonTTY => {
                return Err(miette!("Use --yes to confirm").into());
            }
        }
    }

    Ok(())
}
