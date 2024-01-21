use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::{info, warn};

use ockam::Context;
use ockam_api::cli_state::random_name;
use ockam_api::cloud::enroll::auth0::*;
use ockam_api::cloud::project::{Project, Projects};
use ockam_api::cloud::space::{Space, Spaces};
use ockam_api::cloud::ControllerClient;
use ockam_api::enroll::enrollment::{EnrollStatus, Enrollment};
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::nodes::InMemoryNode;

use crate::enroll::OidcServiceExt;
use crate::node::util::initialize_default_node;
use crate::operation::util::check_for_project_completion;
use crate::output::OutputFormat;
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
    /// The name of an existing identity that you wish to enroll
    #[arg(global = true, value_name = "IDENTITY_NAME", long)]
    pub identity: Option<String>,

    /// Use PKCE authorization flow
    #[arg(long)]
    pub authorization_code_flow: bool,

    /// Skip creation of default Space and default Project
    #[arg(long)]
    pub user_account_only: bool,
}

impl EnrollCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, EnrollCommand)) -> miette::Result<()> {
    if opts.global_args.output_format == OutputFormat::Json {
        return Err(miette::miette!(
            "The flag --output json is invalid for this command."
        ));
    }
    run_impl(&ctx, opts.clone(), cmd).await?;
    initialize_default_node(&ctx, &opts).await?;
    Ok(())
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
    cmd: EnrollCommand,
) -> miette::Result<()> {
    opts.terminal.write_line(&fmt_log!(
        "Enrolling your default Ockam identity with Ockam Orchestrator...\n"
    ))?;

    ctrlc_handler(opts.clone());
    display_parse_logs(&opts);

    let oidc_service = OidcService::default();
    let token = if cmd.authorization_code_flow {
        oidc_service.get_token_with_pkce().await.into_diagnostic()?
    } else {
        oidc_service.get_token_interactively(&opts).await?
    };

    let user_info = oidc_service
        .wait_for_email_verification(&token, Some(&opts.terminal))
        .await?;
    opts.state.store_user(&user_info).await?;

    let identity_name = opts
        .state
        .get_named_identity_or_default(&cmd.identity)
        .await?
        .name();
    let node = InMemoryNode::start_node_with_identity(ctx, &opts.state, &identity_name).await?;
    let controller = node.create_controller().await?;

    enroll_with_node(&controller, ctx, token)
        .await
        .wrap_err("Failed to enroll your local identity with Ockam Orchestrator")?;
    let identifier = node.identifier();
    opts.state
        .set_identifier_as_enrolled(&identifier)
        .await
        .wrap_err("Unable to set the local identity as enrolled")?;
    info!("Enrolled a user with the Identifier {}", identifier);

    if let Err(e) = retrieve_user_project(&opts, ctx, &node, cmd.user_account_only).await {
        warn!(
            "Unable to retrieve your Orchestrator resources. Try running `ockam enroll` again or \
        create them manually using the `ockam space` and `ockam project` commands"
        );
        warn!("{e}");
    }

    opts.terminal.write_line(&fmt_ok!(
        "Enrolled {} as one of the Ockam identities of your Orchestrator account {}.",
        identifier
            .to_string()
            .color(OckamColor::PrimaryResource.color()),
        user_info.email
    ))?;
    Ok(())
}

async fn retrieve_user_project(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    user_account_only: bool,
) -> Result<Project> {
    // return the default project if there is one already stored locally
    if let Ok(project) = opts.state.get_default_project().await {
        return Ok(project);
    };

    let space = get_user_space(opts, ctx, node, user_account_only)
        .await
        .map_err(|e| {
            miette!(
                "Unable to retrieve and set a space as default {:?}",
                e.to_string()
            )
        })?
        .ok_or(miette!("No space was found"))?;

    info!("Retrieved the user default space {:?}", space);

    let project = get_user_project(opts, ctx, node, user_account_only, &space)
        .await
        .wrap_err(format!(
            "Unable to retrieve and set a project as default with space {}",
            space
                .name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))?
        .ok_or(miette!("No project was found"))?;
    info!("Retrieved the user default project {:?}", project);
    Ok(project)
}

/// Enroll a user with a token, using the controller
pub async fn enroll_with_node(
    controller: &ControllerClient,
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

async fn get_user_space(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    user_account_only: bool,
) -> Result<Option<Space>> {
    // return the default space if there is one already stored locally
    if let Ok(space) = opts.state.get_default_space().await {
        return Ok(Some(space));
    };

    // Otherwise get the available spaces for node's identity
    // Those spaces might have been created previously and all the local state reset
    opts.terminal
        .write_line(&fmt_log!("Getting available spaces in your account..."))?;
    let is_finished = Mutex::new(false);
    let get_spaces = async {
        let spaces = node.get_spaces(ctx).await?;
        *is_finished.lock().await = true;
        Ok(spaces)
    };

    let message = vec![format!("Checking for any existing spaces...")];
    let progress_output = opts.terminal.progress_output(&message, &is_finished);

    let (spaces, _) = try_join!(get_spaces, progress_output)?;

    // If the identity has no spaces, create one
    let space = match spaces.first() {
        None => {
            if user_account_only {
                opts.terminal
                    .write_line(&fmt_para!("No spaces are defined in your account."))?;
                return Ok(None);
            }

            opts.terminal
                .write_line(&fmt_para!("No spaces are defined in your account."))?
                .write_line(&fmt_para!(
                    "Provisioning a space for you ({}) ...",
                    "if you don't use it for a few weeks, we'll automatically delete everything in it"
                        .to_string()
                        .color(OckamColor::FmtWARNBackground.color())
                ))?
                .write_line(&fmt_para!(
                    "To learn more about production ready spaces in Ockam Orchestrator, contact us at: {}",
                    "hello@ockam.io".to_string().color(OckamColor::PrimaryResource.color())))?;

            let is_finished = Mutex::new(false);
            let space_name = random_name();
            let create_space = async {
                let space = node.create_space(ctx, &space_name, vec![]).await?;
                *is_finished.lock().await = true;
                Ok(space)
            };

            let message = vec![format!(
                "Creating space {}...",
                space_name
                    .clone()
                    .color(OckamColor::PrimaryResource.color())
            )];
            let progress_output = opts.terminal.progress_output(&message, &is_finished);
            let (space, _) = try_join!(create_space, progress_output)?;
            space
        }
        Some(space) => {
            opts.terminal.write_line(&fmt_log!(
                "Found space {}.",
                space
                    .name
                    .clone()
                    .color(OckamColor::PrimaryResource.color())
            ))?;
            space.clone()
        }
    };
    opts.terminal.write_line(&fmt_ok!(
        "Marked this space as your default space, on this machine.\n"
    ))?;
    Ok(Some(space))
}

async fn get_user_project(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    user_account_only: bool,
    space: &Space,
) -> Result<Option<Project>> {
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
        let projects = node.get_admin_projects(ctx).await?;
        *is_finished.lock().await = true;
        Ok(projects)
    };

    let message = vec![format!("Checking for any existing projects...")];
    let progress_output = opts.terminal.progress_output(&message, &is_finished);

    let (projects, _) = try_join!(get_projects, progress_output)?;

    // If the space has no projects, create one
    let project = match projects.first() {
        None => {
            if user_account_only {
                opts.terminal.write_line(&fmt_para!(
                    "No projects are defined in the space {}.",
                    space
                        .name
                        .to_string()
                        .color(OckamColor::PrimaryResource.color())
                ))?;
                return Ok(None);
            }

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
                let project = node
                    .create_project(ctx, &space.name, &project_name, vec![])
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

            check_for_project_completion(opts, ctx, node, project).await?
        }
        Some(project) => {
            opts.terminal.write_line(&fmt_log!(
                "Found project {}.",
                project
                    .project_name()
                    .color(OckamColor::PrimaryResource.color())
            ))?;
            project.clone()
        }
    };

    let project = check_project_readiness(opts, ctx, node, project).await?;
    // store the updated project
    opts.state.store_project(project.clone()).await?;
    // set the project as the default one
    opts.state.set_default_trust_context(&project.name).await?;
    opts.state.set_default_project(&project.id).await?;

    opts.terminal.write_line(&fmt_ok!(
        "Marked this project as your default project, on this machine.\n"
    ))?;
    Ok(Some(project))
}
