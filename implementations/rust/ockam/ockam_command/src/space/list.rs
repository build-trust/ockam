use clap::Args;
use miette::IntoDiagnostic;

use ockam::Context;
use ockam_api::cli_state::{SpaceConfig, StateDirTrait};
use ockam_api::cloud::space::Space;
use tokio::sync::Mutex;
use tokio::try_join;

use crate::node::util::delete_embedded_node;
use crate::util::api::{self, CloudOpts};
use crate::util::{node_rpc, Rpc};
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List spaces
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct ListCommand {
    #[command(flatten)]
    pub cloud_opts: CloudOpts,
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
    let is_finished: Mutex<bool> = Mutex::new(false);
    let mut rpc = Rpc::embedded(ctx, &opts).await?;

    let send_req = async {
        rpc.request(api::space::list(&cmd.cloud_opts.route()))
            .await?;

        *is_finished.lock().await = true;
        rpc.parse_response::<Vec<Space>>()
    };

    let output_messages = vec![format!("Listing Spaces...\n",)];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (spaces, _) = try_join!(send_req, progress_output)?;

    let plain = opts
        .terminal
        .build_list(&spaces, "Spaces", "No spaces found.")?;
    let json = serde_json::to_string_pretty(&spaces).into_diagnostic()?;

    for space in spaces {
        opts.state
            .spaces
            .overwrite(&space.name, SpaceConfig::from(&space))?;
    }
    delete_embedded_node(&opts, rpc.node_name()).await;

    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;
    Ok(())
}
