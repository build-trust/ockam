use crate::policy::policy_path;

use crate::util::{node_rpc, parse_node_name, Rpc};
use crate::{fmt_ok, CommandGlobalOpts};
use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_abac::{Action, Resource};
use ockam_core::api::Request;

#[derive(Clone, Debug, Args)]
pub struct DeleteCommand {
    #[arg(long, display_order = 900, id = "NODE_NAME")]
    at: String,

    #[arg(short, long)]
    resource: Resource,

    #[arg(short, long)]
    action: Action,

    /// Confirm the deletion without prompting
    #[arg(display_order = 901, long, short)]
    yes: bool,
}

impl DeleteCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, DeleteCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> miette::Result<()> {
    let node = parse_node_name(&cmd.at)?;
    if opts
        .terminal
        .confirmed_with_flag_or_prompt(cmd.yes, "Are you sure you want to delete this policy?")?
    {
        let req = Request::delete(policy_path(&cmd.resource, &cmd.action));
        let mut rpc = Rpc::background(ctx, &opts, &node)?;
        rpc.request(req).await?;
        rpc.is_ok()?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!("Policy with name '{}' has been deleted", &cmd.at))
            .machine(&cmd.at)
            .json(serde_json::json!({ "policy": { "at": &cmd.at } }))
            .write_line()?;
    }
    Ok(())
}
