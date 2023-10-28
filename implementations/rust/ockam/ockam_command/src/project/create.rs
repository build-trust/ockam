use clap::Args;

use ockam::Context;
use ockam_api::cli_state::random_name;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::Projects;
use ockam_api::nodes::InMemoryNode;

use crate::operation::util::check_for_completion;
use crate::project::util::check_project_readiness;
use crate::util::api::CloudOpts;
use crate::util::{api, node_rpc};
use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/create/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/create/after_long_help.txt");

/// Create projects
#[derive(Clone, Debug, Args)]
#[command(
    long_about = docs::about(LONG_ABOUT),
    after_long_help = docs::after_help(AFTER_LONG_HELP),
)]
pub struct CreateCommand {
    /// Name of the Space the project belongs to.
    #[arg(display_order = 1001)]
    pub space_name: String,

    /// Name of the project - must be unique within parent Space
    #[arg(display_order = 1002, default_value_t = random_name(), hide_default_value = true, value_parser = validate_project_name)]
    pub project_name: String,

    #[command(flatten)]
    pub cloud_opts: CloudOpts,
    //TODO:  list of admins
}

impl CreateCommand {
    pub fn run(self, options: CommandGlobalOpts) {
        node_rpc(rpc, (options, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, CreateCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: CreateCommand,
) -> miette::Result<()> {
    let space_id = opts.state.spaces.get(&cmd.space_name)?.config().id.clone();
    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    let project = controller
        .create_project(ctx, space_id, cmd.project_name, vec![])
        .await?;
    let operation_id = project.operation_id.clone().unwrap();
    check_for_completion(&opts, ctx, &controller, &operation_id).await?;
    let project = check_project_readiness(&opts, ctx, &node, project).await?;
    opts.state
        .projects
        .overwrite(&project.name, project.clone())?;
    opts.state
        .trust_contexts
        .overwrite(&project.name, project.clone().try_into()?)?;
    opts.println(&project)?;
    Ok(())
}

fn validate_project_name(s: &str) -> Result<String, String> {
    match api::validate_cloud_resource_name(s) {
        Ok(_) => Ok(s.to_string()),
        Err(_e)=> Err(String::from(
            "project name can contain only alphanumeric characters and the '-', '_' and '.' separators. \
            Separators must occur between alphanumeric characters. This implies that separators can't \
            occur at the start or end of the name, nor they can occur in sequence.",
        )),
    }
}
