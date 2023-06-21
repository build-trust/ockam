use anyhow::anyhow;
use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::tcp::util::alias_parser;
use crate::util::{extract_address_value, node_rpc, Rpc};
use crate::Result;
use crate::{docs, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;
use ockam::Context;
use ockam_core::api::{Request, RequestBuilder};
use crate::terminal::ConfirmResult;

const AFTER_LONG_HELP: &str = include_str!("./static/delete/after_long_help.txt");

/// Delete a TCP Outlet
#[derive(Clone, Debug, Args)]
#[command(after_long_help = docs::after_help(AFTER_LONG_HELP))]
pub struct DeleteCommand {
    /// Name assigned to outlet that will be deleted
    #[arg(display_order = 900, required = true, id = "ALIAS", value_parser = alias_parser)]
    alias: String,

    /// Node on which to stop the tcp outlet. If none are provided, the default node will be used
    #[command(flatten)]
    node_opts: NodeOpts,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(run_impl, (opts, self))
    }
}

pub async fn run_impl(
    ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    let alias = cmd.alias.clone();
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node = extract_address_value(&node_name)?;

    if cmd.yes {
        let mut rpc = Rpc::background(&ctx, &opts, &node)?;
        rpc.request(make_api_request(cmd)?).await?;

        rpc.is_ok()?;

        opts.terminal
            .stdout()
            .plain(format!(
                "{} TCP Outlet with alias {alias} on Node {node} has been deleted.",
                "✔︎".light_green(),
            ))
            .machine(&alias)
            .json(serde_json::json!({ "tcp-outlet": { "alias": alias, "node": node } }))
            .write_line()?;
    } else {
        match opts.terminal.confirm("This will delete the selected Tcp-outlet. Are you sure?")? {
            ConfirmResult::Yes => {
                let mut rpc = Rpc::background(&ctx, &opts, &node)?;
                rpc.request(make_api_request(cmd)?).await?;

                rpc.is_ok()?;

                opts.terminal
                    .stdout()
                    .plain(format!(
                        "{} TCP Outlet with alias {alias} on Node {node} has been deleted.",
                        "✔︎".light_green(),
                    ))
                    .machine(&alias)
                    .json(serde_json::json!({ "tcp-outlet": { "alias": alias, "node": node } }))
                    .write_line()?;
            }
            ConfirmResult::No => {
                return Ok(());
            }
            ConfirmResult::NonTTY => {
                return Err(anyhow!("Use --yes to confirm").into());
            }
        }
    }

    Ok(())
}

/// Construct a request to delete a tcp outlet
fn make_api_request<'a>(cmd: DeleteCommand) -> Result<RequestBuilder<'a>> {
    let alias = cmd.alias;
    let request = Request::delete(format!("/node/outlet/{alias}"));
    Ok(request)
}
