use crate::node::get_node_name;
use crate::terminal::OckamColor;
use crate::util::output::Output;
use crate::util::{node_rpc, parse_node_name, Rpc};
use crate::{CommandGlobalOpts, Result};
use clap::Args;
use colorful::Colorful;
use miette::miette;
use ockam::Context;
use ockam_abac::Resource;
use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::models::policy::{Expression, PolicyList};
use ockam_core::api::Request;
use std::fmt::Write;
use tokio::sync::Mutex;
use tokio::try_join;

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

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, ListCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: ListCommand,
) -> miette::Result<()> {
    let resource = cmd.resource;

    let at = get_node_name(&opts.state, &cmd.at);
    let node_name = parse_node_name(&at)?;

    if !opts.state.nodes.get(&node_name)?.is_running() {
        return Err(miette!("The node '{}' is not running", &node_name));
    }

    let mut rpc = Rpc::background(ctx, &opts, &node_name)?;
    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        let req = Request::get(format!("/policy/{resource}"));

        rpc.request(req).await?;
        rpc.parse_response_body::<PolicyList>()
    };

    let output_messages = vec![format!(
        "Listing Policies on {} for Resource {}...\n",
        node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color()),
        resource
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (policies, _) = try_join!(send_req, progress_output)?;

    let list = opts.terminal.build_list(
        policies.expressions(),
        &format!("Policies on Node {} for {}", &node_name, resource),
        &format!("No Policies on Node {} for {}", &node_name, resource),
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
