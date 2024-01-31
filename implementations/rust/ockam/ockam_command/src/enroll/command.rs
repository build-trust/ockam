use std::collections::HashMap;
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
use ockam_api::journeys::{JourneyEvent, USER_EMAIL, USER_NAME};
use ockam_api::nodes::InMemoryNode;

use crate::enroll::OidcServiceExt;
use crate::fmt_heading;
use crate::operation::util::check_for_project_completion;
use crate::output::OutputFormat;
use crate::project::util::check_project_readiness;
use crate::terminal::{color_email, color_primary, color_uri, OckamColor};
use crate::util::async_cmd;
use crate::{docs, fmt_log, fmt_ok, CommandGlobalOpts, Result};
use crate::{fmt_warn, node::util::initialize_default_node};

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Enroll with Ockam Orchestrator
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct EnrollCommand {
    /// The name of an existing Ockam Identity that you wish to enroll
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
    pub fn run(self, opts: CommandGlobalOpts) -> miette::Result<()> {
        async_cmd(&self.name(), opts.clone(), |ctx| async move {
            self.async_run(&ctx, opts).await
        })
    }

    pub fn name(&self) -> String {
        "enroll".to_string()
    }

    async fn async_run(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        if opts.global_args.output_format == OutputFormat::Json {
            return Err(miette::miette!(
            "This command is interactive and requires you to open a web browser to complete enrollment. \
            Please try running it again without '--output json'."
        ));
        }
        self.run_impl(ctx, opts.clone()).await?;
        initialize_default_node(ctx, &opts).await?;
        Ok(())
    }

    async fn run_impl(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        opts.terminal.write_line(&fmt_log!(
            "{}{}{}",
            "Enrolling your Ockam Identity",
            " on this machine ".dim(),
            "with Ockam Orchestrator."
        ))?;

        ctrlc_handler(opts.clone());

        let oidc_service = OidcService::default();
        let token = if self.authorization_code_flow {
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
            .get_named_identity_or_default(&self.identity)
            .await?
            .name();
        let node = InMemoryNode::start_node_with_identity(ctx, &opts.state, &identity_name).await?;
        let controller = node.create_controller().await?;

        enroll_with_node(&controller, ctx, token)
            .await
            .wrap_err("Failed to enroll your local Identity with Ockam Orchestrator")?;
        let identifier = node.identifier();
        opts.state
            .set_identifier_as_enrolled(&identifier)
            .await
            .wrap_err("Unable to set your local Identity as enrolled")?;
        info!("Enrolled your local Identity with the identifier {identifier}");

        if let Err(e) =
            retrieve_user_space_and_project(&opts, ctx, &node, self.user_account_only).await
        {
            warn!(
            "Unable to retrieve your Orchestrator resources. Try running `ockam enroll` again or \
            create them manually using the `ockam space` and `ockam project` commands."
        );
            warn!("{e}");
        }

        let mut attributes = HashMap::default();
        let user_email = user_info.email.to_string();
        attributes.insert(USER_NAME, user_info.name.as_str());
        attributes.insert(USER_EMAIL, user_email.as_str());
        opts.state
            .add_journey_event(JourneyEvent::Enrolled, attributes)
            .await?;

        // Print final message.
        opts.terminal.write_line(&fmt_ok!(
            "\nEnrolled the following as one of the Identities of your Orchestrator account ({}):",
            color_email(user_info.email.to_string())
        ))?;

        // Print the identity name if it exists.
        if let Ok(named_identity) = opts
            .state
            .get_named_identity_by_identifier(&identifier)
            .await
        {

            if named_identity.is_default() {
                opts.terminal
                    .write_line(&fmt_log!("Existing default identity: '{}' will be used for enrollment. \
                    To use a different identity, run `ockam enroll --identity <IDENTITY_NAME>`.", 
                        color_primary(named_identity.name()))
                    )?;
            }
            else {
                opts.terminal
                    .write_line(&fmt_log!("Chosen identity: '{}' will be used for enrollment. \
                     To use the default identity, run `ockam enroll` instead.", 
                        color_primary(named_identity.name()))
                    )?;
            }
        }

        // Print the identity identifier.
        opts.terminal.write_line(&fmt_log!(
            "identifier: {}\n",
            color_primary(identifier.to_string())
        ))?;
        // Final line.
        opts.terminal.write_line(fmt_log!(
            "{} {}:",
            "Take a look at this tutorial to learn how to securely connect your apps using",
            color_primary("Ockam".to_string())
        ))?;
        opts.terminal.write_line(fmt_log!(
            "{}\n",
            color_uri("https://docs.ockam.io/guides/examples/basic-web-app")
        ))?;

        Ok(())
    }
}

fn ctrlc_handler(opts: CommandGlobalOpts) {
    let is_confirmation = Arc::new(AtomicBool::new(false));
    ctrlc::set_handler(move || {
        if is_confirmation.load(Ordering::Relaxed) {
            let message = fmt_ok!(
                "Received Ctrl+C again. Canceling {}. Please try again.",
                "ockam enroll".bold().light_yellow()
            );
            let _ = opts.terminal.write_line(format!("\n{}", message).as_str());
            process::exit(2);
        } else {
            let message = fmt_warn!(
                "{} is still in progress. Please press Ctrl+C again to stop the enrollment process.",
                "ockam enroll".bold().light_yellow()
            );
            let _ = opts.terminal.write_line(format!("\n{}", message).as_str());
            is_confirmation.store(true, Ordering::Relaxed);
        }
    })
        .expect("Error setting Ctrl-C handler");
}

async fn retrieve_user_space_and_project(
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
                "Unable to retrieve and set a Space as default {:?}",
                e.to_string()
            )
        })?
        .ok_or(miette!("No Space was found"))?;

    info!("Retrieved your default Space {space:#?}");

    let project = get_user_project(opts, ctx, node, user_account_only, &space)
        .await
        .wrap_err(format!(
            "Unable to retrieve and set a Project as default with Space {}",
            color_primary(space.name.to_string())
        ))?
        .ok_or(miette!("No Project was found"))?;
    info!("Retrieved your default Project {project:#?}");
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
        EnrollStatus::UnexpectedStatus(e, s) => warn!("Unexpected status while enrolling: {s}. The error is: {e}."),
        EnrollStatus::FailedNoStatus(e) => warn!("A status was expected in the response to an enrollment request, but got none. The error is: {e}."),
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
        .write_line(&fmt_heading!("Getting available Spaces in your account."))?;
    let is_finished = Mutex::new(false);
    let get_spaces = async {
        let spaces = node.get_spaces(ctx).await?;
        *is_finished.lock().await = true;
        Ok(spaces)
    };

    let message = vec![format!("Checking for any existing Spaces...")];
    let progress_output = opts.terminal.progress_output(&message, &is_finished);

    let (spaces, _) = try_join!(get_spaces, progress_output)?;

    // If the identity has no spaces, create one
    let space = match spaces.first() {
        None => {
            if user_account_only {
                opts.terminal
                    .write_line(&fmt_log!("No Spaces are defined in your account.\n"))?;
                return Ok(None);
            }

            opts.terminal
                .write_line(&fmt_log!("No Spaces are defined in your account, creating a new one.\n"))?
                .write_line(&fmt_log!(
                    "{}",
                    "If you don't use it for a few weeks, we'll delete the Space and Projects within it."
                        .to_string()
                        .color(OckamColor::FmtWARNBackground.color())
                ))?
                .write_line(&fmt_log!(
                    "Interested in deploying Ockam Orchestrator in production? Contact us at: {}.\n",
                    color_email("hello@ockam.io".to_string())
                ))?;

            let is_finished = Mutex::new(false);
            let space_name = random_name();
            let create_space = async {
                let space = node.create_space(ctx, &space_name, vec![]).await?;
                *is_finished.lock().await = true;
                Ok(space)
            };

            let message = vec![format!(
                "Creating a new Space {}...",
                color_primary(space_name.clone())
            )];
            let progress_output = opts.terminal.progress_output(&message, &is_finished);
            let (space, _) = try_join!(create_space, progress_output)?;
            space
        }
        Some(space) => {
            opts.terminal.write_line(&fmt_log!(
                "Found existing Space {}.",
                color_primary(space.name.clone())
            ))?;
            space.clone()
        }
    };
    opts.terminal.write_line(&fmt_ok!(
        "Marked {} as your default Space, {}.",
        color_primary(space.name.clone()),
        "on this machine".dim()
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
    opts.terminal.write_line(&fmt_heading!(
        "Getting available Projects in the Space {}.",
        color_primary(space.name.to_string())
    ))?;

    let is_finished = Mutex::new(false);
    let get_projects = async {
        let projects = node.get_admin_projects(ctx).await?;
        *is_finished.lock().await = true;
        Ok(projects)
    };

    let message = vec![format!("Checking for existing Projects...")];
    let progress_output = opts.terminal.progress_output(&message, &is_finished);

    let (projects, _) = try_join!(get_projects, progress_output)?;

    // If the space has no projects, create one
    let project = match projects.first() {
        None => {
            if user_account_only {
                opts.terminal.write_line(&fmt_log!(
                    "No Projects are defined in the Space {}.",
                    color_primary(space.name.to_string())
                ))?;
                return Ok(None);
            }

            opts.terminal.write_line(&fmt_log!(
                "No Projects are defined in the Space {}. Creating a new one.\n",
                color_primary(space.name.to_string())
            ))?;

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
                "Creating a new Project {}...",
                color_primary(project_name.to_string())
            )];
            let progress_output = opts.terminal.progress_output(&message, &is_finished);
            let (project, _) = try_join!(get_project, progress_output)?;

            opts.terminal.write_line(&fmt_ok!(
                "Created Project {}.",
                color_primary(project_name.to_string())
            ))?;

            check_for_project_completion(opts, ctx, node, project).await?
        }
        Some(project) => {
            opts.terminal.write_line(&fmt_log!(
                "Found Project {}.\n",
                color_primary(project.project_name())
            ))?;
            project.clone()
        }
    };

    let project = check_project_readiness(opts, ctx, node, project).await?;
    // store the updated project
    opts.state.store_project(project.clone()).await?;
    // set the project as the default one
    opts.state.set_default_project(&project.id).await?;

    opts.terminal.write_line(&fmt_ok!(
        "Marked {} as your default Project, {}.\n",
        color_primary(project.project_name()),
        "on this machine".dim()
    ))?;
    Ok(Some(project))
}


