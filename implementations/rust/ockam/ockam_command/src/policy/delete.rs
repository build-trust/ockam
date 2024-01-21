use clap::Args;
use colorful::Colorful;

use ockam::Context;
use ockam_abac::{Action, Resource};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;

use crate::policy::policy_path;
use crate::util::node_rpc;
use crate::{fmt_ok, CommandGlobalOpts};

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

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, DeleteCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: DeleteCommand,
) -> miette::Result<()> {
    if opts
        .terminal
        .confirmed_with_flag_or_prompt(cmd.yes, "Are you sure you want to delete this policy?")?
    {
        let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.at).await?;
        let policy_path = policy_path(&cmd.resource, &cmd.action);
        let req = Request::delete(&policy_path);
        node.tell(ctx, req).await?;

        opts.terminal
            .stdout()
            .plain(fmt_ok!(
                "Policy with path '{}' has been deleted",
                &policy_path
            ))
            .machine(&policy_path)
            .json(serde_json::json!({
                "resource": &cmd.resource.to_string(),
                "action": &cmd.action.to_string(),
                "at": &node.node_name()}
            ))
            .write_line()?;
    }
    Ok(())
}
