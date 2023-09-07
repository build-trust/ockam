use crate::node::get_node_name;
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
    at: Option<String>,

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
    let at = get_node_name(&opts.state, &cmd.at);
    let node_name = parse_node_name(&at)?;
    if opts
        .terminal
        .confirmed_with_flag_or_prompt(cmd.yes, "Are you sure you want to delete this policy?")?
    {
        let policy_path = policy_path(&cmd.resource, &cmd.action);
        let req = Request::delete(&policy_path);
        let mut rpc = Rpc::background(ctx, &opts, &node_name)?;
        rpc.tell(req).await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Policy with path '{}' has been deleted",
                &policy_path
            ))
            .machine(&policy_path)
            .json(serde_json::json!({ "policy": {
                "resource": &cmd.resource.to_string(),
                "action": &cmd.action.to_string(),
                "at": &node_name}
            }))
            .write_line()?;
    }
    Ok(())
}
