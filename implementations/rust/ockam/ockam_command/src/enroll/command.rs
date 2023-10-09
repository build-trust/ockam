use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::Args;
use colorful::Colorful;
use miette::{IntoDiagnostic, WrapErr};
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::info;
use tracing::log::warn;

use ockam::identity::Identifier;
use ockam::Context;
use ockam_api::cli_state::traits::StateDirTrait;
use ockam_api::cli_state::{random_name, update_enrolled_identity, SpaceConfig};
use ockam_api::cloud::enroll::auth0::*;
use ockam_api::cloud::project::{Project, Projects};
use ockam_api::cloud::space::{Space, Spaces};
use ockam_api::cloud::Controller;
use ockam_api::enroll::enrollment::{EnrollStatus, Enrollment};
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::nodes::InMemoryNode;

use crate::enroll::OidcServiceExt;
use crate::identity::initialize_identity_if_default;
use crate::operation::util::check_for_completion;
use crate::project::util::check_project_readiness;
use crate::terminal::OckamColor;
use crate::util::node_rpc;
use crate::{display_parse_logs, docs, fmt_log, fmt_ok, fmt_para, CommandGlobalOpts, Result};

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Enroll with Ockam Orchestrator
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct EnrollCommand {
    /// Run the command as the given identity name
    #[arg(global = true, value_name = "IDENTITY_NAME", long)]
    pub identity: Option<String>,

    /// Use PKCE authorization flow
    #[arg(long)]
    pub authorization_code_flow: bool,
}

impl EnrollCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_identity_if_default(&opts, &self.identity);
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, EnrollCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

fn ctrlc_handler(opts: CommandGlobalOpts) {
    let is_confirmation = Arc::new(AtomicBool::new(false));
    ctrlc::set_handler(move || {
        if is_confirmation.load(Ordering::Relaxed) {
            let _ = opts.terminal.write_line(
                format!(
                    "\n{} Received Ctrl+C again. Cancelling {}. Please try again.",
                    "!".red(), "ockam enroll".bold().light_yellow()
                )
                .as_str(),
            );
            process::exit(2);
        } else {
            let _ = opts.terminal.write_line(
                format!(
                    "\n{} {} is still in progress. If you would like to stop the enrollment process, press Ctrl+C again.",
                    "!".red(), "ockam enroll".bold().light_yellow()
                )
                .as_str(),
            );
            is_confirmation.store(true, Ordering::Relaxed);
        }
    })
    .expect("Error setting Ctrl-C handler");
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    _cmd: EnrollCommand,
) -> miette::Result<()> {
    opts.terminal.write_line(&fmt_log!(
        "Enrolling your default Ockam identity with Ockam Orchestrator...\n"
    ))?;

    ctrlc_handler(opts.clone());
    display_parse_logs(&opts);

    let oidc_service = OidcService::default();
    let token = if _cmd.authorization_code_flow {
        oidc_service.get_token_with_pkce().await.into_diagnostic()?
    } else {
        oidc_service.get_token_interactively(&opts).await?
    };

    let user_info = oidc_service
        .wait_for_email_verification(&token, Some(&opts.terminal))
        .await?;
    opts.state
        .users_info
        .overwrite(&user_info.email, user_info.clone())?;

    let node = InMemoryNode::start(ctx, &opts.state).await?;
    let controller = node.create_controller().await?;

    enroll_with_node(&controller, ctx, token)
        .await
        .wrap_err("Failed to enroll your local identity with Ockam Orchestrator")?;

    let identifier = retrieve_user_project(&opts, ctx, &node).await?;

    opts.terminal.write_line(&fmt_ok!(
        "Enrolled {} as one of the Ockam identities of your Orchestrator account {}.",
        identifier
            .to_string()
            .color(OckamColor::PrimaryResource.color()),
        user_info.email
    ))?;
    Ok(())
}

pub async fn retrieve_user_project(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
) -> Result<Identifier> {
    let space = default_space(opts, ctx, &node.create_controller().await?)
        .await
        .wrap_err("Unable to retrieve and set a space as default")?;
    info!("Retrieved the user default space {:?}", space);

    let project = default_project(opts, ctx, node, &space)
        .await
        .wrap_err(format!(
            "Unable to retrieve and set a project as default with space {}",
            space
                .name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))?;
    info!("Retrieved the user default project {:?}", project);

    let identifier = update_enrolled_identity(&opts.state, &node.node_name())
        .await
        .wrap_err(format!(
            "Unable to set the local identity as enrolled with project {}",
            project
                .name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))?;
    info!("Enrolled a user with the Identifier {}", identifier);

    Ok(identifier)
}

/// Enroll a user with a token, using the controller
pub async fn enroll_with_node(
    controller: &Controller,
    ctx: &Context,
    token: OidcToken,
) -> miette::Result<()> {
    let reply = controller.enroll_with_oidc_token(ctx, token).await?;
    match reply {
        EnrollStatus::EnrolledSuccessfully => info!("Enrolled successfully"),
        EnrollStatus::AlreadyEnrolled => info!("Already enrolled"),
        EnrollStatus::UnexpectedStatus(e, s) => warn!("Unexpected status {s}. The error is: {e}"),
        EnrollStatus::FailedNoStatus(e) => warn!("A status was expected in the response to an enrollment request, got none. The error is: {e}"),
    };
    Ok(())
}

async fn default_space(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    controller: &Controller,
) -> Result<Space> {
    // Get available spaces for node's identity
    opts.terminal
        .write_line(&fmt_log!("Getting available spaces in your account..."))?;
    let is_finished = Mutex::new(false);
    let get_spaces = async {
        let spaces: Vec<Space> = controller.list_spaces(ctx).await?;
        *is_finished.lock().await = true;
        Ok(spaces)
    };

    let message = vec![format!("Checking for any existing spaces...")];
    let progress_output = opts.terminal.progress_output(&message, &is_finished);

    let (mut available_spaces, _) = try_join!(get_spaces, progress_output)?;

    // If the identity has no spaces, create one
    let default_space = if available_spaces.is_empty() {
        opts.terminal
            .write_line(&fmt_para!("No spaces are defined in your account."))?
            .write_line(&fmt_para!(
                "Creating a trial space for you ({}) ...",
                "everything in it will be deleted in 15 days"
                    .to_string()
                    .color(OckamColor::FmtWARNBackground.color())
            ))?
            .write_line(&fmt_para!(
            "To learn more about production ready spaces in Ockam Orchestrator, contact us at: {}",
            "hello@ockam.io".to_string().color(OckamColor::PrimaryResource.color())
        ))?;

        let is_finished = Mutex::new(false);
        let name = random_name();
        let space_name = name.clone();
        let create_space = async {
            let space = controller.create_space(ctx, space_name, vec![]).await?;
            *is_finished.lock().await = true;
            Ok(space)
        };

        let message = vec![format!(
            "Creating space {}...",
            name.color(OckamColor::PrimaryResource.color())
        )];
        let progress_output = opts.terminal.progress_output(&message, &is_finished);
        let (space, _) = try_join!(create_space, progress_output)?;
        space
    }
    // If it has, return the first one on the list
    else {
        for space in &available_spaces {
            opts.state
                .spaces
                .overwrite(&space.name, SpaceConfig::from(space))?;
        }

        let space = available_spaces
            .drain(..1)
            .next()
            .expect("already checked that is not empty");

        opts.terminal.write_line(&fmt_log!(
            "Found space {}.",
            space
                .name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))?;
        space
    };
    opts.state
        .spaces
        .overwrite(&default_space.name, SpaceConfig::from(&default_space))?;
    opts.terminal.write_line(&fmt_ok!(
        "Marked this space as your default space, on this machine.\n"
    ))?;
    Ok(default_space)
}

async fn default_project(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    space: &Space,
) -> Result<Project> {
    let controller = node.create_controller().await?;

    // Get available project for the given space
    opts.terminal.write_line(&fmt_log!(
        "Getting available projects in space {}...",
        space
            .name
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    ))?;

    let is_finished = Mutex::new(false);
    let get_projects = async {
        let projects = controller.list_projects(ctx).await?;
        *is_finished.lock().await = true;
        Ok(projects)
    };

    let message = vec![format!("Checking for any existing projects...")];
    let progress_output = opts.terminal.progress_output(&message, &is_finished);

    let (mut available_projects, _) = try_join!(get_projects, progress_output)?;

    // If the space has no projects, create one
    let default_project = if available_projects.is_empty() {
        opts.terminal
            .write_line(&fmt_para!(
                "No projects are defined in the space {}.",
                space
                    .name
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ))?
            .write_line(&fmt_para!("Creating a project for you..."))?;

        let is_finished = Mutex::new(false);
        let project_name = "default".to_string();
        let get_project = async {
            let project = controller
                .create_project(ctx, space.id.clone(), project_name.clone(), vec![])
                .await?;
            *is_finished.lock().await = true;
            Ok(project)
        };

        let message = vec![format!(
            "Creating project {}...",
            project_name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        )];
        let progress_output = opts.terminal.progress_output(&message, &is_finished);
        let (project, _) = try_join!(get_project, progress_output)?;

        opts.terminal.write_line(&fmt_ok!(
            "Created project {}.",
            project_name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))?;

        let operation_id = project.operation_id.clone().unwrap();
        check_for_completion(opts, ctx, &controller, &operation_id).await?;

        project.to_owned()
    }
    // If it has, return the "default" project or first one on the list
    else {
        for project in &available_projects {
            opts.state
                .projects
                .overwrite(&project.name, project.clone())?;
        }
        let p = match available_projects.iter().find(|ns| ns.name == "default") {
            None => available_projects
                .drain(..1)
                .next()
                .expect("already checked that is not empty"),
            Some(p) => p.to_owned(),
        };
        opts.terminal.write_line(&fmt_log!(
            "Found project {}.",
            p.name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))?;
        p
    };

    let project = check_project_readiness(opts, ctx, node, default_project).await?;

    opts.terminal.write_line(&fmt_ok!(
        "Marked this project as your default project, on this machine.\n"
    ))?;

    opts.state
        .projects
        .overwrite(&project.name, project.clone())?;
    opts.state
        .trust_contexts
        .overwrite(&project.name, project.clone().try_into()?)?;
    Ok(project)
}
