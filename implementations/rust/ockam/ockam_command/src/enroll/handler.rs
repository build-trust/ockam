use async_trait::async_trait;
use colorful::Colorful;
use miette::{miette, IntoDiagnostic, WrapErr};
use r3bl_rs_utils_core::UnicodeString;
use r3bl_tui::{
    ColorWheel, ColorWheelConfig, ColorWheelSpeed, GradientGenerationPolicy, TextColorizationPolicy,
};
use std::collections::HashMap;
use std::io::stdin;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::{error, info, instrument, warn, Level};

use crate::enroll::OidcServiceExt;
use crate::error::Error;
use crate::node_command::InMemoryNodeCommand;
use crate::operation::util::check_for_project_completion;
use crate::project::util::check_project_readiness;
use crate::{CommandGlobalOpts, Result};
use ockam::Context;
use ockam_api::cli_state::journeys::{JourneyEvent, USER_EMAIL, USER_NAME};
use ockam_api::colors::{color_primary, color_uri, color_warn, OckamColor};
use ockam_api::enroll::enrollment::{EnrollStatus, Enrollment};
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::nodes::InMemoryNode;
use ockam_api::orchestrator::enroll::auth0::*;
use ockam_api::orchestrator::project::Project;
use ockam_api::orchestrator::project::ProjectsOrchestratorApi;
use ockam_api::orchestrator::space::{Space, Spaces};
use ockam_api::orchestrator::subscription::subscription_page;
use ockam_api::orchestrator::ControllerClient;
use ockam_api::terminal::notification::NotificationHandler;
use ockam_api::{fmt_err, fmt_log, fmt_ok, fmt_separator, fmt_warn};

#[derive(Clone)]
pub struct EnrollHandler {
    pub opts: CommandGlobalOpts,
    pub identity_name: Option<String>,
    pub authorization_code_flow: bool,
    pub force: bool,
    pub skip_orchestrator_resources_creation: bool,
    pub enable_ctrlc_signal: bool,
    pub is_ai_cloud_account: bool,
}

#[async_trait]
impl InMemoryNodeCommand for EnrollHandler {
    async fn init(&self) -> miette::Result<()> {
        self.ctrlc_handler();

        if self.is_already_enrolled().await? {
            return Ok(());
        }

        self.display_header();

        let _notification_handler =
            NotificationHandler::start(self.opts.state.clone(), self.opts.terminal.clone());
        self.opts
            .state
            .get_named_identity_or_default(&self.identity_name)
            .await?;
        Ok(())
    }

    async fn run(&self, node: Arc<InMemoryNode>) -> miette::Result<()> {
        let (user_info, cluster) = self.enroll_identity(&node).await?;

        if !self.is_ai_cloud_account {
            if let Err(error) = self.retrieve_user_space_and_project(&node).await {
                // Display output to user
                self.opts.terminal
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
        }

        // Tracing
        let mut attributes = HashMap::new();
        attributes.insert(USER_NAME, user_info.name.clone());
        attributes.insert(USER_EMAIL, user_info.email.to_string());
        // this event formally only happens on the host journey
        // but we add it here for better rendering of the project journey
        self.opts
            .state
            .add_journey_event(JourneyEvent::ok("enroll".to_string()), attributes.clone())
            .await?;
        self.opts
            .state
            .add_journey_event(JourneyEvent::Enrolled, attributes)
            .await?;

        let identity = self
            .opts
            .state
            .get_named_identity_or_default(&self.identity_name)
            .await?;

        // Output
        self.opts.terminal.write_line(fmt_log!(
            "Your Identity {}, with Identifier {} is now enrolled with Ockam Orchestrator.",
            color_primary(identity.name()),
            color_primary(identity.identifier().to_string())
        ))?;
        if let Some(cluster) = cluster {
            self.opts.terminal.write_line(fmt_log!(
                "Your Cluster associated to the Ockam AI Platform is {cluster}"
            ))?;
        }

        if !self.is_ai_cloud_account {
            self.opts.terminal
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
        }
        Ok(())
    }
}

impl EnrollHandler {
    // Creates one span in the trace
    #[instrument(
        skip_all, // Drop all args that passed in, as Context doesn't play nice
        fields(
        enroller = ? self.identity_name, // https://docs.rs/tracing/latest/tracing/
        authorization_code_flow = % self.authorization_code_flow,
        force = % self.force,
        skip_orchestrator_resources_creation = % self.skip_orchestrator_resources_creation,
        ), level = Level::TRACE)]
    pub async fn handle(&self, ctx: &Context) -> miette::Result<()> {
        if self.opts.global_args.output_format().is_json() {
            return Err(miette::miette!(
                "This command is interactive and requires you to open a web browser to complete enrollment. \
                Please try running it again without '--output json'."
            ));
        }
        self.execute(ctx, self.opts.state.clone()).await?;
        Ok(())
    }

    /// Check if the identity is already enrolled and display a message to the user.
    pub async fn is_already_enrolled(&self) -> miette::Result<bool> {
        let mut is_already_enrolled = !self
            .opts
            .state
            .identity_should_enroll(&self.identity_name, false)
            .await?;
        if is_already_enrolled {
            match &self.identity_name {
                // Use default identity.
                None => {
                    if let Ok(named_identity) =
                        self.opts.state.get_or_create_default_named_identity().await
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
                            self.opts.terminal.write_line(fmt_log!("{}", line)).unwrap();
                        });
                    }
                }
                // Identity specified.
                Some(ref name) => {
                    let named_identity = self.opts.state.get_named_identity(name).await?;
                    let name = named_identity.name();
                    let identifier = named_identity.identifier();
                    let message = format!(
                        "Your Identity {}\nwith Identifier {}\nis already enrolled as one of the Identities associated with your Ockam account.",
                        color_primary(name),
                        color_primary(identifier.to_string())
                    );
                    message.split('\n').for_each(|line| {
                        self.opts.terminal.write_line(fmt_log!("{}", line)).unwrap();
                    });
                }
            };
        }

        // Check if the default space is available and has a valid subscription
        let default_space = match self.opts.state.get_default_space().await {
            Ok(space) => space,
            Err(_) => {
                // If there is no default space, we want to continue with the enrollment process
                return Ok(false);
            }
        };
        is_already_enrolled &= default_space.has_valid_subscription();

        Ok(is_already_enrolled)
    }

    pub(crate) async fn enroll_identity(
        &self,
        node: &InMemoryNode,
    ) -> miette::Result<(UserInfo, Option<String>)> {
        if !self
            .opts
            .state
            .identity_should_enroll(&self.identity_name, self.force)
            .await?
        {
            if let Ok(user_info) = self.opts.state.get_default_user().await {
                return Ok((user_info, None));
            }
        }

        self.opts.terminal.write_line(fmt_log!(
            "Enrolling your Identity with Ockam Orchestrator..."
        ))?;

        // Run OIDC service
        let oidc_service = OidcService::new()?;
        let token = if self.authorization_code_flow {
            oidc_service.get_token_with_pkce().await.into_diagnostic()?
        } else {
            oidc_service.get_token_interactively(&self.opts).await?
        };

        // Store user info retrieved from OIDC service
        let user_info = oidc_service
            .wait_for_email_verification(&token, Some(&self.opts.terminal))
            .await?;
        self.opts.state.store_user(&user_info).await?;

        // Enroll the identity with the Orchestrator
        let controller = node.create_controller().await?;
        let cluster = self
            .enroll_with_node(node.ctx(), &controller, token)
            .await
            .wrap_err("Failed to enroll your local Identity with Ockam Orchestrator")?;
        self.opts
            .state
            .set_identifier_as_enrolled(&node.identifier(), &user_info.email)
            .await
            .wrap_err("Unable to set your local Identity as enrolled")?;

        Ok((user_info, cluster))
    }

    fn display_header(&self) {
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

        let _ = self
            .opts
            .terminal
            .write_line(format!("{}\n", colored_header));
    }

    fn ctrlc_handler(&self) {
        if !self.enable_ctrlc_signal {
            return;
        }

        let is_confirmation = Arc::new(AtomicBool::new(false));
        let terminal = self.opts.terminal.clone();
        ctrlc::set_handler(move || {
            if is_confirmation.load(Ordering::Relaxed) {
                let message = fmt_ok!(
                "Received Ctrl+C again. Canceling {}. Please try again.",
                "ockam enroll".bold().light_yellow()
            );
                let _ = terminal.write_line(format!("\n{}", message).as_str());
                process::exit(2);
            } else {
                let message = fmt_warn!(
                "{} is still in progress. Please press Ctrl+C again to stop the enrollment process.",
                "ockam enroll".bold().light_yellow()
            );
                let _ = terminal.write_line(format!("\n{}", message).as_str());
                is_confirmation.store(true, Ordering::Relaxed);
            }
        })
            .expect("Error setting Ctrl-C handler");
    }

    #[instrument(skip_all, level = Level::TRACE)]
    async fn retrieve_user_space_and_project(
        &self,
        node: &InMemoryNode,
    ) -> miette::Result<Project> {
        self.opts.terminal.write_line(fmt_separator!())?;
        let space = self
            .get_user_space(node)
            .await
            .wrap_err("Unable to retrieve and set a Space as default")?
            .ok_or(miette!("No Space was found"))?;
        let project = self
            .get_user_project(node, &space)
            .await
            .wrap_err(format!(
                "Unable to retrieve and set a Project as default with Space {}",
                color_primary(&space.name)
            ))?
            .ok_or(miette!("No Project was found"))?;
        self.opts.terminal.write_line(fmt_separator!())?;
        Ok(project)
    }

    /// Enroll a user with a token, using the controller
    async fn enroll_with_node(
        &self,
        ctx: &Context,
        controller: &ControllerClient,
        token: OidcToken,
    ) -> miette::Result<Option<String>> {
        let cluster: Option<String> = None;
        let reply = if self.is_ai_cloud_account {
            let reply = controller.enroll_ai_with_oidc_token(ctx, token).await?;
            // if let AiEnrollStatus::EnrolledSuccessfully(c) = &reply {
            //     cluster = Some(c.to_string());
            // }
            // cluster = Some(controller.get_cluster(ctx).await?.into_inner()); // TODO: remove once enroll_ai_with_oidc_token is fixed
            reply.into()
        } else {
            controller.enroll_with_oidc_token(ctx, token).await?
        };
        match reply {
            EnrollStatus::EnrolledSuccessfully => {
                info!("Enrolled successfully");
                Ok(cluster)
            }
            EnrollStatus::AlreadyEnrolled => {
                info!("Already enrolled");
                Ok(cluster)
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

    async fn get_user_space(&self, node: &InMemoryNode) -> miette::Result<Option<Space>> {
        // Get the available spaces for node's identity
        self.opts.terminal.write_line(fmt_log!(
            "Getting available Spaces accessible to your account."
        ))?;

        let spaces = {
            let sp = self.opts.terminal.spinner();
            if let Some(spinner) = sp.as_ref() {
                spinner.set_message("Checking for any existing Spaces...");
            }
            node.get_spaces().await?
        };

        let subscription_page = subscription_page()?.to_string();

        let space = match spaces.first() {
            // If the identity has no spaces, create one
            None => {
                // send user to subscription page
                self.opts
                    .terminal
                    .write_line(fmt_log!("No Spaces are accessible to your account.\n"))?;
                self.opts.terminal.write_line(fmt_log!(
                    "Please go to {} and subscribe to create a new Space.",
                    color_uri(&subscription_page)
                ))?;

                if self.skip_orchestrator_resources_creation {
                    return Ok(None);
                }

                self.ask_user_to_subscribe_and_wait_for_space_to_be_ready(node)
                    .await?
            }
            Some(space) => {
                self.opts.terminal.write_line(fmt_log!(
                    "Found existing Space {}.\n",
                    color_primary(&space.name)
                ))?;
                match &space.subscription {
                    // if no subscription is attached to the space, ask the user to subscribe
                    None => {
                        self.opts.terminal.write_line(fmt_log!(
                            "Your Space {} doesn't have a Subscription attached to it.",
                            color_primary(&space.name)
                        ))?;
                        self.opts.terminal.write_line(fmt_log!(
                            "Please go to {} and subscribe to use your Space.",
                            color_uri(&subscription_page)
                        ))?;
                        self.ask_user_to_subscribe_and_wait_for_space_to_be_ready(node)
                            .await?
                    }
                    Some(subscription) => {
                        // if there is a subscription, check that it's not expired
                        if !subscription.is_valid() {
                            self.opts.terminal.write_line(fmt_log!(
                                "Your Trial of the {} Subscription on the Space {} has ended.",
                                subscription.name.colored(),
                                color_primary(&space.name)
                            ))?;
                            self.opts.terminal.write_line(fmt_log!(
                            "Please go to {} and subscribe to one of our paid plans to use your Space.",
                            color_uri(&subscription_page)
                        ))?;
                            if let Some(grace_period_end_date) =
                                subscription.grace_period_end_date()?
                            {
                                let date =
                                    grace_period_end_date.format_human().into_diagnostic()?;
                                let msg = if grace_period_end_date.is_in_the_past() {
                                    format!("All Projects in this Space were deleted on {date}.")
                                } else {
                                    format!("All Projects in this Space will be deleted on {date}.")
                                };
                                self.opts
                                    .terminal
                                    .write_line(fmt_log!("{}", color_warn(msg)))?;
                            }
                            self.ask_user_to_subscribe_and_wait_for_space_to_be_ready(node)
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
                color_uri(&subscription_page)
            )
            .wrap_err("The Space does not have a subscription plan attached.")
        })?;
        self.opts.terminal.write_line(fmt_ok!(
            "Marked {} as your default Space, on this machine.\n",
            color_primary(&space.name)
        ))?;
        if let Ok(msg) = space.subscription_status_message() {
            self.opts.terminal.write_line(msg)?;
        }
        Ok(Some(space))
    }

    async fn ask_user_to_subscribe_and_wait_for_space_to_be_ready(
        &self,
        node: &InMemoryNode,
    ) -> Result<Space> {
        let subscription_page = subscription_page()?.to_string();

        self.opts.terminal.write_line("")?;
        if self.opts.terminal.can_ask_for_user_input() {
            self.opts.terminal.write(fmt_log!(
                "Press {} to open {} in your browser.",
                " ENTER ↵ ".bg_white().black().blink(),
                color_uri(&subscription_page)
            ))?;

            let mut input = String::new();
            match stdin().read_line(&mut input) {
                Ok(_) => {
                    self.opts
                        .terminal
                        .write_line(fmt_log!("Opening your browser..."))?;
                }
                Err(_e) => {
                    return Err(miette!(
                        "Couldn't read user input or enter keypress from stdin"
                    ))?;
                }
            }
        }
        if open::that(&subscription_page).is_err() {
            self.opts.terminal.write_line(fmt_err!(
                "Couldn't open your browser from the terminal. Please open {} manually.",
                color_uri(&subscription_page)
            ))?;
        }

        self.opts.terminal.write_line("")?;

        // wait until the user has subscribed and a space is created
        let sp = self.opts.terminal.spinner();
        if let Some(spinner) = sp.as_ref() {
            let msg = "Waiting for you to subscribe using your browser...";
            spinner.set_message(msg);
        }
        let space = loop {
            let spaces = node.get_spaces().await?;
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
        &self,
        node: &InMemoryNode,
        space: &Space,
    ) -> Result<Option<Project>> {
        // Get available projects for the given space
        self.opts.terminal.write_line(fmt_log!(
            "Getting available Projects in the Space {}...",
            color_primary(&space.name)
        ))?;

        let projects = {
            let sp = self.opts.terminal.spinner();
            if let Some(spinner) = sp.as_ref() {
                spinner.set_message("Checking for any existing Projects...");
            }
            node.get_admin_projects().await?
        };

        // If the space has no projects, create one
        let project = match projects.first() {
            None => {
                if self.skip_orchestrator_resources_creation {
                    self.opts.terminal.write_line(fmt_log!(
                        "No Project is defined in the Space {}.",
                        color_primary(&space.name)
                    ))?;
                    return Ok(None);
                }

                self.opts.terminal.write_line(fmt_log!(
                    "No Project is defined in the Space {}, creating a new one...",
                    color_primary(&space.name)
                ))?;

                let is_finished = Mutex::new(false);
                let project_name = "default".to_string();
                let get_project = async {
                    let project = node
                        .create_project(&space.name, &project_name, vec![])
                        .await?;
                    *is_finished.lock().await = true;
                    Ok(project)
                };

                let message = vec![format!(
                    "Creating a new Project {}...",
                    color_primary(&project_name)
                )];
                let progress_output = self.opts.terminal.loop_messages(&message, &is_finished);
                let (project, _) = try_join!(get_project, progress_output)?;

                self.opts.terminal.write_line(fmt_ok!(
                    "Created a new Project named {}.",
                    color_primary(&project_name)
                ))?;

                check_for_project_completion(&self.opts, node, project).await?
            }
            Some(project) => {
                self.opts.terminal.write_line(fmt_log!(
                    "Found Project named {}.",
                    color_primary(project.name())
                ))?;

                project.clone()
            }
        };

        let project = check_project_readiness(&self.opts, node, project).await?;
        // store the updated project
        self.opts
            .state
            .projects()
            .store_project(project.clone())
            .await?;

        self.opts.terminal.write_line(fmt_ok!(
            "Marked this new Project as your default Project, on this machine."
        ))?;
        Ok(Some(project))
    }
}
