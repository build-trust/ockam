use std::collections::HashMap;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::{error, info, instrument, warn};

use ockam::Context;
use ockam_api::cli_state::random_name;
use ockam_api::cloud::enroll::auth0::*;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::project::ProjectsOrchestratorApi;
use ockam_api::cloud::space::{Space, Spaces};
use ockam_api::cloud::ControllerClient;
use ockam_api::enroll::enrollment::{EnrollStatus, Enrollment};
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::journeys::{JourneyEvent, USER_EMAIL, USER_NAME};
use ockam_api::nodes::InMemoryNode;
use ockam_api::CliState;

use crate::enroll::OidcServiceExt;
use crate::error::Error;
use crate::operation::util::check_for_project_completion;
use crate::output::OutputFormat;
use crate::progress_display::ProgressDisplay;
use crate::project::util::check_project_readiness;
use crate::terminal::{color_primary, color_uri, OckamColor};
use crate::util::async_cmd;
use crate::{docs, fmt_heading, fmt_log, fmt_ok, fmt_warn, CommandGlobalOpts, Result};

use r3bl_rs_utils_core::UnicodeString;
use r3bl_tui::{
    ColorWheel, ColorWheelConfig, ColorWheelSpeed, GradientGenerationPolicy, TextColorizationPolicy,
};

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Enroll your Ockam Identity with Ockam Orchestrator
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct EnrollCommand {
    /// The name of an existing Ockam Identity that you wish to enroll. You can use `ockam
    /// identity list` to get a list of existing Identities. To create a new Identity, use
    /// `ockam identity create`. If you don't specify an Identity, and you don't have a
    /// default Identity, this command will create a default Identity for you and save it
    /// locally in a default Vault
    #[arg(global = true, value_name = "IDENTITY_NAME", long)]
    pub identity: Option<String>,

    /// This option allows you to bypass pasting the one-time code and confirming device
    /// activation, and PKCE (Proof Key for Code Exchange) authorization flow. Please be
    /// careful with this option since it will open your default system browser. This
    /// option might be useful if you have already enrolled and want to re-enroll using
    /// the same account information
    #[arg(long)]
    pub authorization_code_flow: bool,

    /// By default this command skips the enrollment process if the Identity you specified
    /// (using `--identity`), or the default Identity, is already enrolled, by checking
    /// its status. Use this flag to force the execution of the Identity enrollment
    /// process.
    #[arg(long)]
    pub force: bool,

    /// Use this flag to skip creating Orchestrator resources. When you use this flag, we
    /// only check whether the Orchestrator resources are created. And if they are not, we
    /// will continue without creating them.
    #[arg(hide = true, long = "skip-resource-creation", conflicts_with = "force")]
    pub skip_orchestrator_resources_creation: bool,
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
        Ok(())
    }

    // Creates one span in the trace
    #[instrument(
    skip_all, // Drop all args that passed in, as Context doesn't play nice
    fields(
        enroller = ? self.identity, // https://docs.rs/tracing/latest/tracing/
        authorization_code_flow = % self.authorization_code_flow,
        force = % self.force,
        skip_orchestrator_resources_creation = % self.skip_orchestrator_resources_creation,
    ))]
    async fn run_impl(&self, ctx: &Context, mut opts: CommandGlobalOpts) -> miette::Result<()> {
        ctrlc_handler(opts.clone());

        if self.is_already_enrolled(&opts.state, &opts).await? {
            return Ok(());
        }

        display_header(&opts);

        let identity = {
            let _progress_display = ProgressDisplay::start(&opts);
            opts.state
                .get_named_identity_or_default(&self.identity)
                .await?
        };

        let identity_name = identity.name();
        let identifier = identity.identifier();
        let node = InMemoryNode::start_node_with_identity(ctx, &opts.state, &identity_name).await?;

        let user_info = self.enroll_identity(ctx, &opts, &node).await?;

        if let Err(error) = retrieve_user_space_and_project(
            &opts,
            ctx,
            &node,
            self.skip_orchestrator_resources_creation,
        )
        .await
        {
            // Display output to user
            opts.terminal
                .write_line("")?
                .write_line(&fmt_warn!(
                    "There was a problem retrieving your space and project: {}",
                    color_primary(error.to_string())
                ))?
                .write_line(&fmt_log!(
                    "If this problem persists, please report this issue, with a copy of your logs, to {}\n",
                    color_uri("https://github.com/build-trust/ockam/issues")
                ))?;

            // Log output to operator
            error!(
                "Unable to retrieve your Orchestrator resources. Try running `ockam enroll` again or \
                create them manually using the `ockam space` and `ockam project` commands."
            );
            error!("{error}");

            // Exit the command with an error
            return Err(error.wrap_err(format!(
                "There was a problem, please try to enroll again using {}.",
                color_primary("ockam enroll")
            )));
        }

        // Tracing
        let mut attributes = HashMap::new();
        attributes.insert(USER_NAME, user_info.name.clone());
        attributes.insert(USER_EMAIL, user_info.email.to_string());
        // this event formally only happens on the host journey
        // but we add it here for better rendering of the project journey
        opts.state
            .add_journey_event(JourneyEvent::ok("enroll".to_string()), attributes.clone())
            .await?;
        opts.state
            .add_journey_event(JourneyEvent::Enrolled, attributes)
            .await?;

        // Output
        opts.terminal
            .write_line(&fmt_log!(
                "Your Identity {}, with Identifier {} is now enrolled with Ockam Orchestrator.",
                color_primary(identity_name),
                color_primary(identifier.to_string())
            ))?
            .write_line(&fmt_log!(
                "You also now have an Orchestrator Project that offers a Project Membership Authority service and a Relay service.\n"
            ))?
            .write_line(&fmt_log!(
                "Please explore our documentation to learn how you can use Ockam"
            ))?
            .write_line(&fmt_log!(
                "to create encrypted Portals to remote services, databases, and more {}",
                color_uri("https://docs.ockam.io")
            ))?;

        Ok(())
    }

    /// Check if the identity is already enrolled and display a message to the user.
    async fn is_already_enrolled(
        &self,
        cli_state: &CliState,
        opts: &CommandGlobalOpts,
    ) -> miette::Result<bool> {
        let is_already_enrolled = !cli_state
            .identity_should_enroll(&self.identity, false)
            .await?;
        if is_already_enrolled {
            match &self.identity {
                // Use default identity.
                None => {
                    if let Ok(named_identity) =
                        cli_state.get_or_create_default_named_identity().await
                    {
                        let name = named_identity.name();
                        let identifier = named_identity.identifier();
                        let message = format!(
                            "Your {} Identity {}\nwith Identifier {}\nis already enrolled as one of the Identities associated with your Ockam account.",
                            "default".to_string().dim(),
                            color_primary(name),
                            color_primary(identifier.to_string())
                        );
                        message.split('\n').for_each(|line| {
                            opts.terminal.write_line(&fmt_log!("{}", line)).unwrap();
                        });
                    }
                }
                // Identity specified.
                Some(ref name) => {
                    let named_identity = cli_state.get_named_identity(name).await?;
                    let name = named_identity.name();
                    let identifier = named_identity.identifier();
                    let message = format!(
                        "Your Identity {}\nwith Identifier {}\nis already enrolled as one of the Identities associated with your Ockam account.",
                        color_primary(name),
                        color_primary(identifier.to_string())
                    );
                    message.split('\n').for_each(|line| {
                        opts.terminal.write_line(&fmt_log!("{}", line)).unwrap();
                    });
                }
            };
        }
        Ok(is_already_enrolled)
    }

    async fn enroll_identity(
        &self,
        ctx: &Context,
        opts: &CommandGlobalOpts,
        node: &InMemoryNode,
    ) -> miette::Result<UserInfo> {
        if !opts
            .state
            .identity_should_enroll(&self.identity, self.force)
            .await?
        {
            if let Ok(user_info) = opts.state.get_default_user().await {
                return Ok(user_info);
            }
        }

        opts.terminal.write_line(&fmt_log!(
            "Enrolling your Identity with Ockam Orchestrator..."
        ))?;

        // Run OIDC service
        let oidc_service = OidcService::default();
        let token = if self.authorization_code_flow {
            oidc_service.get_token_with_pkce().await.into_diagnostic()?
        } else {
            oidc_service.get_token_interactively(opts).await?
        };

        // Store user info retrieved from OIDC service
        let user_info = oidc_service
            .wait_for_email_verification(&token, Some(&opts.terminal))
            .await?;
        opts.state.store_user(&user_info).await?;

        // Enroll the identity with the Orchestrator
        let controller = node.create_controller().await?;
        enroll_with_node(&controller, ctx, token)
            .await
            .wrap_err("Failed to enroll your local Identity with Ockam Orchestrator")?;
        opts.state
            .set_identifier_as_enrolled(&node.identifier(), &user_info.email)
            .await
            .wrap_err("Unable to set your local Identity as enrolled")?;

        Ok(user_info)
    }
}

fn display_header(opts: &CommandGlobalOpts) {
    let ockam_header = include_str!("../../static/ockam_ascii.txt").trim();
    let gradient_steps = Vec::from(
        [
            OckamColor::OckamBlue.value(),
            OckamColor::HeaderGradient.value(),
        ]
        .map(String::from),
    );
    let colored_header = ColorWheel::new(vec![ColorWheelConfig::Rgb(
        gradient_steps,
        ColorWheelSpeed::Medium,
        50,
    )])
    .colorize_into_string(
        &UnicodeString::from(ockam_header),
        GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
        TextColorizationPolicy::ColorEachCharacter(None),
    );

    let _ = opts.terminal.write_line(&format!("{}\n", colored_header));
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

#[instrument(skip_all)]
async fn retrieve_user_space_and_project(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    skip_orchestrator_resources_creation: bool,
) -> miette::Result<Project> {
    opts.terminal.write_line(fmt_heading!(""))?;
    let space = get_user_space(opts, ctx, node, skip_orchestrator_resources_creation)
        .await
        .wrap_err("Unable to retrieve and set a Space as default")?
        .ok_or(miette!("No Space was found"))?;

    info!("Retrieved your default Space {space:#?}");

    let project = get_user_project(
        opts,
        ctx,
        node,
        skip_orchestrator_resources_creation,
        &space,
    )
    .await
    .wrap_err(format!(
        "Unable to retrieve and set a Project as default with Space {}",
        color_primary(&space.name)
    ))?
    .ok_or(miette!("No Project was found"))?;
    info!("Retrieved your default Project {project:#?}");
    opts.terminal.write_line(fmt_heading!(""))?;
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
        EnrollStatus::EnrolledSuccessfully => {
            info!("Enrolled successfully");
            Ok(())
        }
        EnrollStatus::AlreadyEnrolled => {
            info!("Already enrolled");
            Ok(())
        }
        EnrollStatus::UnexpectedStatus(error, status) => {
            warn!(%error, %status, "Unexpected status while enrolling");
            Err(Error::new_internal_error(&error).into())
        }
        EnrollStatus::FailedNoStatus(error) => {
            warn!(%error, "A status was expected in the response to an enrollment request, but got none");
            Err(Error::new_internal_error(&error).into())
        }
    }
}

async fn get_user_space(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    skip_orchestrator_resources_creation: bool,
) -> miette::Result<Option<Space>> {
    // Get the available spaces for node's identity
    // Those spaces might have been created previously and all the local state reset
    opts.terminal
        .write_line(&fmt_log!("Getting available Spaces in your account."))?;
    let is_finished = Mutex::new(false);
    let get_spaces = async {
        let spaces = node.get_spaces(ctx).await?;
        *is_finished.lock().await = true;
        Ok(spaces)
    };

    let message = vec!["Checking for any existing Spaces...".to_string()];
    let progress_output = opts.terminal.progress_output(&message, &is_finished);

    let (spaces, _) = try_join!(get_spaces, progress_output)?;

    // If the identity has no spaces, create one
    let space = match spaces.first() {
        None => {
            if skip_orchestrator_resources_creation {
                opts.terminal
                    .write_line(&fmt_log!("No Spaces are defined in your account.\n"))?;
                return Ok(None);
            }

            opts.terminal.write_line(&fmt_log!(
                "No Spaces are defined in your account, creating a new one..."
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
            opts.terminal.write_line(&fmt_ok!(
                "Created a new Space named {}.",
                color_primary(space.name.clone())
            ))?;
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
        "Marked {} as your default Space, {}.\n",
        color_primary(space.name.clone()),
        "on this machine".dim()
    ))?;

    opts.terminal.write_line(fmt_log!("This Space does not have a Subscription attached to it."))?
        .write_line(fmt_log!("As a courtesy, we created a temporary Space for you, so you can continue to build.\n"))?
        .write_line(fmt_log!("Please subscribe to an Ockam plan within two weeks {}", color_uri("https://www.ockam.io/pricing")))?
        .write_line(fmt_log!("{}\n", "If you don't subscribe in that time, your Space and all Projects will be permanently deleted.".color(OckamColor::FmtWARNBackground.color())))?;

    Ok(Some(space))
}

async fn get_user_project(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    skip_orchestrator_resources_creation: bool,
    space: &Space,
) -> Result<Option<Project>> {
    // Get available project for the given space
    opts.terminal.write_line(&fmt_log!(
        "Getting available Projects in the Space {}...",
        color_primary(&space.name)
    ))?;

    let is_finished = Mutex::new(false);
    let get_projects = async {
        let projects = node.get_admin_projects(ctx).await?;
        *is_finished.lock().await = true;
        Ok(projects)
    };

    let message = vec!["Checking for existing Projects...".to_string()];
    let progress_output = opts.terminal.progress_output(&message, &is_finished);

    let (projects, _) = try_join!(get_projects, progress_output)?;

    // If the space has no projects, create one
    let project = match projects.first() {
        None => {
            if skip_orchestrator_resources_creation {
                opts.terminal.write_line(&fmt_log!(
                    "No Project is defined in the Space {}.",
                    color_primary(&space.name)
                ))?;
                return Ok(None);
            }

            opts.terminal.write_line(&fmt_log!(
                "No Project is defined in the Space {}, creating a new one...",
                color_primary(&space.name)
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
                color_primary(&project_name)
            )];
            let progress_output = opts.terminal.progress_output(&message, &is_finished);
            let (project, _) = try_join!(get_project, progress_output)?;

            opts.terminal.write_line(&fmt_ok!(
                "Created a new Project named {}.",
                color_primary(&project_name)
            ))?;

            check_for_project_completion(opts, ctx, node, project).await?
        }
        Some(project) => {
            opts.terminal.write_line(&fmt_log!(
                "Found Project named {}.",
                color_primary(project.name())
            ))?;

            project.clone()
        }
    };

    let project = check_project_readiness(opts, ctx, node, project).await?;
    // store the updated project
    opts.state.projects().store_project(project.clone()).await?;
    // set the project as the default one
    opts.state
        .projects()
        .set_default_project(project.project_id())
        .await?;

    opts.terminal.write_line(&fmt_ok!(
        "Marked this new Project as your default Project, {}.",
        "on this machine".dim()
    ))?;
    Ok(Some(project))
}
