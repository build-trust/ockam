use std::collections::HashMap;
use std::io::stdin;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use clap::Args;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use r3bl_rs_utils_core::UnicodeString;
use r3bl_tui::{
    ColorWheel, ColorWheelConfig, ColorWheelSpeed, GradientGenerationPolicy, TextColorizationPolicy,
};
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::{error, info, instrument, warn};

use crate::enroll::OidcServiceExt;
use crate::error::Error;
use crate::operation::util::check_for_project_completion;
use crate::project::util::check_project_readiness;
use crate::util::async_cmd;
use crate::{docs, CommandGlobalOpts, Result};
use ockam::Context;
use ockam_api::cli_state::journeys::{JourneyEvent, USER_EMAIL, USER_NAME};
use ockam_api::cloud::enroll::auth0::*;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::project::ProjectsOrchestratorApi;
use ockam_api::cloud::space::{Space, Spaces};
use ockam_api::cloud::subscription::SUBSCRIPTION_PAGE;
use ockam_api::cloud::ControllerClient;
use ockam_api::colors::{color_primary, color_uri, color_warn, OckamColor};
use ockam_api::enroll::enrollment::{EnrollStatus, Enrollment};
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::nodes::InMemoryNode;
use ockam_api::terminal::notification::NotificationHandler;
use ockam_api::{fmt_err, fmt_log, fmt_ok, fmt_warn};
use ockam_api::{fmt_separator, CliState};

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

#[derive(Clone, Debug, Args)]
#[command(
about = docs::about("Enroll your Ockam Identity with Ockam Orchestrator"),
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct EnrollCommand {
    #[arg(global = true, value_name = "IDENTITY_NAME", long)]
    #[arg(help = docs::about("\
    The name of an existing Ockam Identity that you wish to enroll. \
    You can use `ockam identity list` to get a list of existing Identities. \
    To create a new Identity, use `ockam identity create`. \
    If you don't specify an Identity name, and you don't have a default Identity, this command \
    will create a default Identity for you and save it locally in the default Vault
    "))]
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
        if opts.global_args.output_format().is_json() {
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
    async fn run_impl(&self, ctx: &Context, opts: CommandGlobalOpts) -> miette::Result<()> {
        ctrlc_handler(opts.clone());

        if self.is_already_enrolled(&opts.state, &opts).await? {
            return Ok(());
        }

        display_header(&opts);

        let identity = {
            let _notification_handler =
                NotificationHandler::start(&opts.state, opts.terminal.clone());
            opts.state
                .get_named_identity_or_default(&self.identity)
                .await?
        };

        let identity_name = identity.name();
        let identifier = identity.identifier();
        let node = InMemoryNode::start_with_identity(ctx, &opts.state, Some(identity_name.clone()))
            .await?;

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
                .write_line(fmt_warn!(
                    "There was a problem retrieving your space and project: {}",
                    color_primary(error.to_string())
                ))?
                .write_line(fmt_log!(
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
            .write_line(fmt_log!(
                "Your Identity {}, with Identifier {} is now enrolled with Ockam Orchestrator.",
                color_primary(identity_name),
                color_primary(identifier.to_string())
            ))?
            .write_line(fmt_log!(
                "You also now have an Orchestrator Project that offers a Project Membership Authority service and a Relay service.\n"
            ))?
            .write_line(fmt_log!(
                "Please explore our documentation to learn how you can use Ockam"
            ))?
            .write_line(fmt_log!(
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
        let mut is_already_enrolled = !cli_state
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
                            opts.terminal.write_line(fmt_log!("{}", line)).unwrap();
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
                        opts.terminal.write_line(fmt_log!("{}", line)).unwrap();
                    });
                }
            };
        }

        // Check if the default space is available and has a valid subscription
        let default_space = match cli_state.get_default_space().await {
            Ok(space) => space,
            Err(_) => {
                // If there is no default space, we want to continue with the enrollment process
                return Ok(false);
            }
        };
        is_already_enrolled &= default_space.has_valid_subscription();

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

        opts.terminal.write_line(fmt_log!(
            "Enrolling your Identity with Ockam Orchestrator..."
        ))?;

        // Run OIDC service
        let oidc_service = OidcService::new()?;
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

    let _ = opts.terminal.write_line(format!("{}\n", colored_header));
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
    opts.terminal.write_line(fmt_separator!())?;
    let space = get_user_space(opts, ctx, node, skip_orchestrator_resources_creation)
        .await
        .wrap_err("Unable to retrieve and set a Space as default")?
        .ok_or(miette!("No Space was found"))?;
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
    opts.terminal.write_line(fmt_separator!())?;
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
    opts.terminal.write_line(fmt_log!(
        "Getting available Spaces accessible to your account."
    ))?;

    let spaces = {
        let sp = opts.terminal.spinner();
        if let Some(spinner) = sp.as_ref() {
            spinner.set_message("Checking for any existing Spaces...");
        }
        node.get_spaces(ctx).await?
    };

    let space = match spaces.first() {
        // If the identity has no spaces, create one
        None => {
            // send user to subscription page
            opts.terminal
                .write_line(fmt_log!("No Spaces are accessible to your account.\n"))?;
            opts.terminal.write_line(fmt_log!(
                "Please go to {} and subscribe to create a new Space.",
                color_uri(SUBSCRIPTION_PAGE)
            ))?;

            if skip_orchestrator_resources_creation {
                return Ok(None);
            }

            ask_user_to_subscribe_and_wait_for_space_to_be_ready(opts, ctx, node).await?
        }
        Some(space) => {
            opts.terminal.write_line(fmt_log!(
                "Found existing Space {}.\n",
                color_primary(&space.name)
            ))?;
            match &space.subscription {
                // if no subscription is attached to the space, ask the user to subscribe
                None => {
                    opts.terminal.write_line(fmt_log!(
                        "Your Space {} doesn't have a Subscription attached to it.",
                        color_primary(&space.name)
                    ))?;
                    opts.terminal.write_line(fmt_log!(
                        "Please go to {} and subscribe to use your Space.",
                        color_uri(SUBSCRIPTION_PAGE)
                    ))?;
                    ask_user_to_subscribe_and_wait_for_space_to_be_ready(opts, ctx, node).await?
                }
                Some(subscription) => {
                    // if there is a subscription, check that it's not expired
                    if !subscription.is_valid() {
                        opts.terminal.write_line(fmt_log!(
                            "Your Trial of the {} Subscription on the Space {} has ended.",
                            subscription.name.colored(),
                            color_primary(&space.name)
                        ))?;
                        opts.terminal.write_line(fmt_log!(
                            "Please go to {} and subscribe to one of our paid plans to use your Space.",
                            color_uri(SUBSCRIPTION_PAGE)
                        ))?;
                        if let Some(grace_period_end_date) = subscription.grace_period_end_date()? {
                            let date = grace_period_end_date.format_human().into_diagnostic()?;
                            let msg = if grace_period_end_date.is_in_the_past() {
                                format!("All Projects in this Space were deleted on {date}.")
                            } else {
                                format!("All Projects in this Space will be deleted on {date}.")
                            };
                            opts.terminal.write_line(fmt_log!("{}", color_warn(msg)))?;
                        }
                        ask_user_to_subscribe_and_wait_for_space_to_be_ready(opts, ctx, node)
                            .await?
                    }
                    // otherwise return the space as is
                    else {
                        space.clone()
                    }
                }
            }
        }
    };
    space.subscription.as_ref().ok_or_else(|| {
        // At this point, the space should have a subscription, but just in case
        miette!(
            "Please go to {} and try again",
            color_uri(SUBSCRIPTION_PAGE)
        )
        .wrap_err("The Space does not have a subscription plan attached.")
    })?;
    opts.terminal.write_line(fmt_ok!(
        "Marked {} as your default Space, on this machine.\n",
        color_primary(&space.name)
    ))?;
    if let Ok(msg) = space.subscription_status_message() {
        opts.terminal.write_line(msg)?;
    }
    Ok(Some(space))
}

async fn ask_user_to_subscribe_and_wait_for_space_to_be_ready(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
) -> Result<Space> {
    opts.terminal.write_line("")?;
    if opts.terminal.can_ask_for_user_input() {
        opts.terminal.write(fmt_log!(
            "Press {} to open {} in your browser.",
            " ENTER ↵ ".bg_white().black().blink(),
            color_uri(SUBSCRIPTION_PAGE)
        ))?;

        let mut input = String::new();
        match stdin().read_line(&mut input) {
            Ok(_) => {
                opts.terminal
                    .write_line(fmt_log!("Opening your browser..."))?;
            }
            Err(_e) => {
                return Err(miette!(
                    "Couldn't read user input or enter keypress from stdin"
                ))?;
            }
        }
    }
    if open::that(SUBSCRIPTION_PAGE).is_err() {
        opts.terminal.write_line(fmt_err!(
            "Couldn't open your browser from the terminal. Please open {} manually.",
            color_uri(SUBSCRIPTION_PAGE)
        ))?;
    }

    opts.terminal.write_line("")?;

    // wait until the user has subscribed and a space is created
    let sp = opts.terminal.spinner();
    if let Some(spinner) = sp.as_ref() {
        let msg = "Waiting for you to subscribe using your browser...";
        spinner.set_message(msg);
    }
    let space = loop {
        let spaces = node.get_spaces(ctx).await?;
        if let Some(space) = spaces.into_iter().next() {
            if space.has_valid_subscription() {
                break space;
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    };
    Ok(space)
}

async fn get_user_project(
    opts: &CommandGlobalOpts,
    ctx: &Context,
    node: &InMemoryNode,
    skip_orchestrator_resources_creation: bool,
    space: &Space,
) -> Result<Option<Project>> {
    // Get available projects for the given space
    opts.terminal.write_line(fmt_log!(
        "Getting available Projects in the Space {}...",
        color_primary(&space.name)
    ))?;

    let projects = {
        let sp = opts.terminal.spinner();
        if let Some(spinner) = sp.as_ref() {
            spinner.set_message("Checking for any existing Projects...");
        }
        node.get_admin_projects(ctx).await?
    };

    // If the space has no projects, create one
    let project = match projects.first() {
        None => {
            if skip_orchestrator_resources_creation {
                opts.terminal.write_line(fmt_log!(
                    "No Project is defined in the Space {}.",
                    color_primary(&space.name)
                ))?;
                return Ok(None);
            }

            opts.terminal.write_line(fmt_log!(
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
            let progress_output = opts.terminal.loop_messages(&message, &is_finished);
            let (project, _) = try_join!(get_project, progress_output)?;

            opts.terminal.write_line(fmt_ok!(
                "Created a new Project named {}.",
                color_primary(&project_name)
            ))?;

            check_for_project_completion(opts, ctx, node, project).await?
        }
        Some(project) => {
            opts.terminal.write_line(fmt_log!(
                "Found Project named {}.",
                color_primary(project.name())
            ))?;

            project.clone()
        }
    };

    let project = check_project_readiness(opts, ctx, node, project).await?;
    // store the updated project
    opts.state.projects().store_project(project.clone()).await?;

    opts.terminal.write_line(fmt_ok!(
        "Marked this new Project as your default Project, on this machine."
    ))?;
    Ok(Some(project))
}
