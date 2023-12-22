use std::fmt::Write;

use clap::Args;
use colorful::Colorful;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_abac::Resource;
use ockam_api::nodes::models::policy::{Expression, PolicyList};
use ockam_api::nodes::BackgroundNodeClient;
use ockam_core::api::Request;

use crate::output::Output;
use crate::terminal::OckamColor;
use crate::util::node_rpc;
use crate::{CommandGlobalOpts, Result};

#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[arg(long, display_order = 900, id = "NODE_NAME")]
    at: Option<String>,

    #[arg(short, long)]
    resource: Resource,
}

impl ListCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, ListCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: ListCommand) -> miette::Result<()> {
    let node = BackgroundNodeClient::create(ctx, &opts.state, &cmd.at).await?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let resource = cmd.resource;
    let get_policies = async {
        let req = Request::get(format!("/policy/{resource}"));
        let policies: PolicyList = node.ask(ctx, req).await?;
        Ok(policies)
    };

    let output_messages = vec![format!(
        "Listing Policies on {} for Resource {}...\n",
        node.node_name()
            .to_string()
            .color(OckamColor::PrimaryResource.color()),
        resource
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (policies, _) = try_join!(get_policies, progress_output)?;

    let list = opts.terminal.build_list(
        policies.expressions(),
        &format!("Policies on Node {} for {}", &node.node_name(), resource),
        &format!("No Policies on Node {} for {}", &node.node_name(), resource),
    )?;
    opts.terminal.stdout().plain(list).write_line()?;

    Ok(())
}

impl Output for Expression {
    fn output(&self) -> Result<String> {
        let mut output = String::new();
        writeln!(
            output,
            "Action: {}",
            self.action()
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;
        write!(
            output,
            "Expression: {}",
            self.expr()
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;
        Ok(output)
    }
}
