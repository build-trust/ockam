use clap::Args;
use miette::IntoDiagnostic;
use tokio::sync::Mutex;
use tokio::try_join;

use ockam::Context;
use ockam_api::cli_state::{SpaceConfig, StateDirTrait};
use ockam_api::cloud::space::{Space, Spaces};

use crate::node::util::{delete_embedded_node, start_node_manager};
use crate::util::api::CloudOpts;
use crate::util::node_rpc;
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/list/long_about.txt");
const PREVIEW_TAG: &str = include_str!("../static/preview_tag.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/list/after_long_help.txt");

/// List spaces
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    before_help = docs::before_help(PREVIEW_TAG),
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
    _cmd: ListCommand,
) -> miette::Result<()> {
    let is_finished: Mutex<bool> = Mutex::new(false);
    let node_manager = start_node_manager(&ctx, &opts, None).await?;
    let controller = node_manager
        .make_controller_client()
        .await
        .into_diagnostic()?;

    let get_spaces = async {
        let spaces: Vec<Space> = controller
            .list_spaces(ctx)
            .await
            .into_diagnostic()?
            .success()
            .into_diagnostic()?;
        *is_finished.lock().await = true;
        Ok(spaces)
    };

    let output_messages = vec![format!("Listing Spaces...\n",)];

    let progress_output = opts
        .terminal
        .progress_output(&output_messages, &is_finished);

    let (spaces, _) = try_join!(get_spaces, progress_output)?;

    let plain = opts
        .terminal
        .build_list(&spaces, "Spaces", "No spaces found.")?;
    let json = serde_json::to_string_pretty(&spaces).into_diagnostic()?;

    for space in spaces {
        opts.state
            .spaces
            .overwrite(&space.name, SpaceConfig::from(&space))?;
    }
    delete_embedded_node(&opts, &node_manager.node_name()).await;

    opts.terminal
        .stdout()
        .plain(plain)
        .json(json)
        .write_line()?;
    Ok(())
}
