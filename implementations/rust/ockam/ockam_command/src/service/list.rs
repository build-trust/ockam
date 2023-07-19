use std::fmt::Write;

use clap::Args;
use colorful::Colorful;
use miette::miette;
use miette::IntoDiagnostic;
use ockam::{Context, TcpTransport};

use ockam_api::cli_state::StateDirTrait;
use ockam_api::nodes::models::services::{ServiceList, ServiceStatus};
use tokio::sync::Mutex;
use tokio::try_join;

use crate::node::{get_node_name, initialize_node_if_default, NodeOpts};
use crate::terminal::OckamColor;
use crate::util::output::Output;
use crate::util::{api, node_rpc, parse_node_name, RpcBuilder};
use crate::CommandGlobalOpts;

/// List service(s) of a given node
#[derive(Clone, Debug, Args)]
pub struct ListCommand {
    #[command(flatten)]
    pub node_opts: NodeOpts,
}

impl ListCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_node_if_default(&opts, &self.node_opts.at_node);
        node_rpc(rpc, (opts, self));
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
    let node_name = get_node_name(&opts.state, &cmd.node_opts.at_node);
    let node_name = parse_node_name(&node_name)?;

    if !opts.state.nodes.get(&node_name)?.is_running() {
        return Err(miette!("The node '{}' is not running", node_name));
    }

    let tcp = TcpTransport::create(ctx).await.into_diagnostic()?;
    let mut rpc = RpcBuilder::new(ctx, &opts, &node_name).tcp(&tcp)?.build();
    let is_finished: Mutex<bool> = Mutex::new(false);

    let send_req = async {
        rpc.request(api::list_services()).await?;
        let r = rpc.parse_response::<ServiceList>()?;

        *is_finished.lock().await = true;
        crate::Result::Ok(r)
    };

    let output_messages = vec![format!(
        "Listing Services on {}...\n",
        node_name
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    )];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (services, _) = try_join!(send_req, progress_output)?;

    let plain = opts.terminal.build_list(
        &services.list,
        &format!("Services on {}", node_name),
        &format!("No services found on {}", node_name),
    )?;
    let json = serde_json::to_string_pretty(&services.list).into_diagnostic()?;
    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;

    Ok(())
}

impl Output for ServiceStatus {
    fn output(&self) -> crate::Result<String> {
        let mut output = String::new();

        writeln!(
            output,
            "Service {}",
            self.service_type
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;
        write!(
            output,
            "Address {}{}",
            "/service/"
                .to_string()
                .color(OckamColor::PrimaryResource.color()),
            self.addr
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )?;

        Ok(output)
    }
}
