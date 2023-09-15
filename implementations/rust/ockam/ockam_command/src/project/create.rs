use clap::Args;
use miette::IntoDiagnostic;
use rand::prelude::random;

use ockam::Context;
use ockam_api::cli_state::{StateDirTrait, StateItemTrait};
use ockam_api::cloud::project::Projects;

use crate::node::util::{delete_embedded_node, start_node_manager};
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
    #[arg(display_order = 1002, default_value_t = hex::encode(&random::<[u8;4]>()), hide_default_value = true, value_parser = validate_project_name)]
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

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, CreateCommand),
) -> miette::Result<()> {
    run_impl(&mut ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &mut Context,
    opts: CommandGlobalOpts,
    cmd: CreateCommand,
) -> miette::Result<()> {
    let space_id = opts.state.spaces.get(&cmd.space_name)?.config().id.clone();
    let node_manager = start_node_manager(&ctx, &opts, None).await?;
    let controller = node_manager
        .make_controller_client()
        .await
        .into_diagnostic()?;

    let project = controller
        .create_project(ctx, space_id, cmd.project_name, vec![])
        .await
        .into_diagnostic()?
        .success()
        .into_diagnostic()?;
    let operation_id = project.operation_id.clone().unwrap();
    check_for_completion(&opts, ctx, &controller, &operation_id).await?;
    let project = check_project_readiness(&opts, ctx, &node_manager, project).await?;
    opts.state
        .projects
        .overwrite(&project.name, project.clone())?;
    opts.state
        .trust_contexts
        .overwrite(&project.name, project.clone().try_into()?)?;
    opts.println(&project)?;
    delete_embedded_node(&opts, &node_manager.node_name()).await;
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
