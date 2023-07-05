use clap::Args;

use miette::{miette, WrapErr};
use ockam_identity::IdentityIdentifier;
use tokio::sync::Mutex;
use tokio::try_join;

use std::borrow::Borrow;
use std::io::stdin;

use colorful::Colorful;
use reqwest::StatusCode;
use tokio::time::{sleep, Duration};
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::{debug, info};

use ockam::Context;
use ockam_api::cli_state::traits::{StateDirTrait, StateItemTrait};
use ockam_api::cli_state::SpaceConfig;
use ockam_api::cloud::enroll::auth0::*;
use ockam_api::cloud::project::{OktaAuth0, Project};
use ockam_api::cloud::space::Space;
use ockam_core::api::Status;

use crate::node::util::{delete_embedded_node, start_embedded_node};
use crate::operation::util::check_for_completion;
use crate::project::util::check_project_readiness;

use crate::terminal::OckamColor;
use crate::util::api::CloudOpts;

use crate::identity::initialize_identity_if_default;
use crate::util::{api, node_rpc, RpcBuilder};
use crate::{
    display_parse_logs, docs, fmt_err, fmt_log, fmt_ok, fmt_para, CommandGlobalOpts, Result,
};

const LONG_ABOUT: &str = include_str!("./static/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/after_long_help.txt");

/// Enroll with Ockam Orchestrator
#[derive(Clone, Debug, Args)]
#[command(
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct EnrollCommand {
    #[command(flatten)]
    pub cloud_opts: CloudOpts,
}

impl EnrollCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_identity_if_default(&opts, &self.cloud_opts.identity);
        node_rpc(rpc, (opts, self));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, EnrollCommand)) -> miette::Result<()> {
    run_impl(&ctx, opts, cmd).await
}

async fn run_impl(
    ctx: &Context,
    opts: CommandGlobalOpts,
    cmd: EnrollCommand,
) -> miette::Result<()> {
    opts.terminal.write_line(&fmt_log!(
        "Enrolling your default Ockam identity with Ockam Orchestrator...\n"
    ))?;

    display_parse_logs(&opts);

    let node_name = start_embedded_node(ctx, &opts, None).await?;

    enroll(ctx, &opts, &cmd, &node_name)
        .await
        .wrap_err("Failed to enroll your local identity with Ockam Orchestrator")?;

    let cloud_opts = cmd.cloud_opts.clone();
    let space = default_space(ctx, &opts, &cloud_opts, &node_name)
        .await
        .wrap_err("Unable to retrieve and set a space as default")?;
    let project = default_project(ctx, &opts, &cloud_opts, &node_name, &space)
        .await
        .wrap_err(format!(
            "Unable to retrieve and set a project as default with space {}",
            space
                .name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))?;
    let identifier = update_enrolled_identity(&opts, &node_name)
        .await
        .wrap_err(format!(
            "Unable to set the local identity as enrolled with project {}",
            project
                .name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))?;
    delete_embedded_node(&opts, &node_name).await;

    opts.terminal.write_line(&fmt_ok!(
        "Enrolled {} as one of the Ockam identities of your Orchestrator account.",
        identifier
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    ))?;
    Ok(())
}

async fn enroll(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    cmd: &EnrollCommand,
    node_name: &str,
) -> miette::Result<()> {
    let auth0 = Auth0Service::new(Auth0Provider::Auth0);
    let token = auth0.token(&cmd.cloud_opts, opts).await?;
    let mut rpc = RpcBuilder::new(ctx, opts, node_name).build();
    rpc.request(api::enroll::auth0(cmd.clone(), token)).await?;
    let (res, dec) = rpc.check_response()?;
    if res.status() == Some(Status::Ok) {
        info!("Enrolled successfully");
        Ok(())
    } else if res.status() == Some(Status::BadRequest) {
        info!("Already enrolled");
        Ok(())
    } else {
        Err(miette!("{}", rpc.parse_err_msg(res, dec)))
    }
}

async fn default_space<'a>(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    cloud_opts: &CloudOpts,
    node_name: &str,
) -> Result<Space<'a>> {
    // Get available spaces for node's identity
    opts.terminal
        .write_line(&fmt_log!("Getting available spaces in your account..."))?;

    let mut rpc = RpcBuilder::new(ctx, opts, node_name).build();
    let is_finished = Mutex::new(false);
    let send_req = async {
        rpc.request(api::space::list(&cloud_opts.route())).await?;
        *is_finished.lock().await = true;
        rpc.parse_response::<Vec<Space>>()
    };

    let message = vec![format!("Checking for any existing spaces...")];
    let progress_output = opts.terminal.progress_output(&message, &is_finished);

    let (mut available_spaces, _) = try_join!(send_req, progress_output)?;

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
        let name = crate::util::random_name();
        let mut rpc = RpcBuilder::new(ctx, opts, node_name).build();
        let send_req = async {
            let cmd = crate::space::CreateCommand {
                cloud_opts: cloud_opts.clone(),
                name: name.to_string(),
                admins: vec![],
            };

            rpc.request(api::space::create(&cmd)).await?;
            *is_finished.lock().await = true;
            rpc.parse_response::<Space>()
        };

        let message = vec![format!(
            "Creating space {}...",
            name.to_string().color(OckamColor::PrimaryResource.color())
        )];
        let progress_output = opts.terminal.progress_output(&message, &is_finished);
        let (space, _) = try_join!(send_req, progress_output)?;
        space.to_owned()
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
            .expect("already checked that is not empty")
            .to_owned();

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
    ctx: &Context,
    opts: &CommandGlobalOpts,
    cloud_opts: &CloudOpts,
    node_name: &str,
    space: &Space<'_>,
) -> Result<Project> {
    // Get available project for the given space
    opts.terminal.write_line(&fmt_log!(
        "Getting available projects in space {}...",
        space
            .name
            .to_string()
            .color(OckamColor::PrimaryResource.color())
    ))?;

    let mut rpc = RpcBuilder::new(ctx, opts, node_name).build();
    let is_finished = Mutex::new(false);
    let send_req = async {
        rpc.request(api::project::list(&cloud_opts.route())).await?;
        *is_finished.lock().await = true;
        rpc.parse_response::<Vec<Project>>()
    };

    let message = vec![format!("Checking for any existing projects...")];
    let progress_output = opts.terminal.progress_output(&message, &is_finished);

    let (mut available_projects, _) = try_join!(send_req, progress_output)?;

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
        let name = "default";
        let mut rpc = RpcBuilder::new(ctx, opts, node_name).build();
        let send_req = async {
            rpc.request(api::project::create(name, &space.id, &cloud_opts.route()))
                .await?;
            *is_finished.lock().await = true;
            rpc.parse_response::<Project>()
        };

        let message = vec![format!(
            "Creating project {}...",
            name.to_string().color(OckamColor::PrimaryResource.color())
        )];
        let progress_output = opts.terminal.progress_output(&message, &is_finished);
        let (project, _) = try_join!(send_req, progress_output)?;

        opts.terminal.write_line(&fmt_ok!(
            "Created project {}.",
            project
                .name
                .to_string()
                .color(OckamColor::PrimaryResource.color())
        ))?;

        let operation_id = project.operation_id.clone().unwrap();
        check_for_completion(ctx, opts, cloud_opts, rpc.node_name(), &operation_id).await?;

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
                .expect("already checked that is not empty")
                .to_owned(),
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
    let project =
        check_project_readiness(ctx, opts, cloud_opts, node_name, None, default_project).await?;

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

pub enum Auth0Provider {
    Auth0,
    Okta(OktaAuth0),
}

impl Auth0Provider {
    fn client_id(&self) -> &str {
        match self {
            Self::Auth0 => "c1SAhEjrJAqEk6ArWjGjuWX11BD2gK8X",
            Self::Okta(d) => &d.client_id,
        }
    }

    const fn scopes(&self) -> &'static str {
        "profile openid email"
    }

    fn device_code_url(&self) -> String {
        match self {
            Self::Auth0 => "https://account.ockam.io/oauth/device/code".to_string(),
            // See https://developer.okta.com/docs/reference/api/oidc/#composing-your-base-url
            Self::Okta(d) => format!("{}/v1/device/authorize", &d.tenant_base_url),
        }
    }

    fn token_request_url(&self) -> String {
        match self {
            Self::Auth0 => "https://account.ockam.io/oauth/token".to_string(),
            Self::Okta(d) => format!("{}/v1/token", &d.tenant_base_url),
        }
    }

    fn build_http_client(&self) -> Result<reqwest::Client> {
        let client = match self {
            Self::Auth0 => reqwest::Client::new(),
            Self::Okta(d) => {
                let certificate = reqwest::Certificate::from_pem(d.certificate.as_bytes())
                    .map_err(|e| miette!("Error parsing certificate: {}", e))?;
                reqwest::ClientBuilder::new()
                    .tls_built_in_root_certs(false)
                    .add_root_certificate(certificate)
                    .build()
                    .map_err(|e| miette!("Error building http client: {}", e))?
            }
        };
        Ok(client)
    }
}

pub struct Auth0Service(Auth0Provider);

impl Auth0Service {
    pub fn new(provider: Auth0Provider) -> Self {
        Self(provider)
    }

    fn provider(&self) -> &Auth0Provider {
        &self.0
    }

    pub(crate) async fn token(
        &self,
        _cloud_opts: &CloudOpts,
        opts: &CommandGlobalOpts,
    ) -> Result<Auth0Token> {
        let dc = self.device_code().await?;

        opts.terminal
            .write_line(&fmt_log!(
                "To enroll we need to associate your Ockam identity with an Orchestrator account:\n"
            ))?
            .write_line(&fmt_para!(
                "First copy this one-time code: {}",
                format!(" {} ", dc.user_code).bg_white().black()
            ))?
            .write(fmt_para!(
                "Then press {} to open {} in your browser.",
                " ENTER ↵ ".bg_white().black().blink(),
                dc.verification_uri
                    .to_string()
                    .color(OckamColor::PrimaryResource.color())
            ))?;

        let mut input = String::new();
        match stdin().read_line(&mut input) {
            Ok(_) => {
                opts.terminal
                    .write_line(&fmt_log!(""))?
                    .write_line(&fmt_para!(
                        "Opening {}, in your browser, to begin authentication...",
                        dc.verification_uri
                            .to_string()
                            .color(OckamColor::PrimaryResource.color())
                    ))?;
            }
            Err(_e) => {
                return Err(miette!("couldn't read enter from stdin").into());
            }
        }

        // Request device activation
        // Note that we try to open the verification uri **without** the code.
        // After the code is entered, if the user closes the tab (because they
        // want to open it on another browser, for example), the uri gets
        // invalidated and the user would have to restart the process (i.e.
        // rerun the command).
        let uri: &str = dc.verification_uri.borrow();
        if open::that(uri).is_err() {
            opts.terminal.write_line(&fmt_err!(
                "Couldn't open activation url automatically [url={}]",
                uri.to_string().light_green()
            ))?;
        }

        self.poll_token(dc, opts).await
    }

    /// Request device code
    pub async fn device_code(&self) -> Result<DeviceCode<'_>> {
        // More on how to use scope and audience in https://auth0.com/docs/quickstart/native/device#device-code-parameters
        let client = self.provider().build_http_client()?;
        let req = || {
            client
                .post(self.provider().device_code_url())
                .header("content-type", "application/x-www-form-urlencoded")
                .form(&[
                    ("client_id", self.provider().client_id()),
                    ("scope", self.provider().scopes()),
                ])
        };
        let retry_strategy = ExponentialBackoff::from_millis(10).take(3);
        let res = Retry::spawn(retry_strategy, move || req().send())
            .await
            .map_err(|e| miette!(e.to_string()))?;
        match res.status() {
            StatusCode::OK => {
                let res = res
                    .json::<DeviceCode>()
                    .await
                    .map_err(|e| miette!(e.to_string()))?;
                debug!(?res, "device code received: {res:#?}");
                Ok(res)
            }
            _ => {
                let res = res.text().await.map_err(|e| miette!(e.to_string()))?;
                let err_msg = "couldn't get device code";
                debug!(?res, err_msg);
                Err(miette!(err_msg).into())
            }
        }
    }

    /// Poll for token until it's ready
    pub async fn poll_token<'a>(
        &'a self,
        dc: DeviceCode<'a>,
        opts: &CommandGlobalOpts,
    ) -> Result<Auth0Token> {
        let client = self.provider().build_http_client()?;
        let token;
        let spinner_option = opts.terminal.progress_spinner();
        if let Some(spinner) = spinner_option.as_ref() {
            spinner.set_message("Waiting for you to complete authentication using your browser...");
        }
        loop {
            let res = client
                .post(self.provider().token_request_url())
                .header("content-type", "application/x-www-form-urlencoded")
                .form(&[
                    ("client_id", self.provider().client_id()),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", &dc.device_code),
                ])
                .send()
                .await
                .map_err(|e| miette!(e.to_string()))?;
            match res.status() {
                StatusCode::OK => {
                    token = res
                        .json::<Auth0Token>()
                        .await
                        .map_err(|e| miette!(e.to_string()))?;
                    debug!(?token, "token response received");
                    if let Some(spinner) = spinner_option.as_ref() {
                        spinner.finish_and_clear();
                    }
                    opts.terminal.write_line(&fmt_para!("Authenticated\n"))?;
                    return Ok(token);
                }
                _ => {
                    let err = res
                        .json::<TokensError>()
                        .await
                        .map_err(|e| miette!(e.to_string()))?;
                    match err.error.borrow() {
                        "authorization_pending" | "invalid_request" | "slow_down" => {
                            debug!(?err, "tokens not yet received");
                            sleep(Duration::from_secs(dc.interval as u64)).await;
                            continue;
                        }
                        _ => {
                            let err_msg = "failed to receive tokens";
                            debug!(?err, "{err_msg}");
                            return Err(miette!(err_msg).into());
                        }
                    }
                }
            }
        }
    }

    pub(crate) async fn validate_provider_config(&self) -> miette::Result<()> {
        if let Err(e) = self.device_code().await {
            return Err(miette!("Invalid OIDC configuration: {}", e));
        }
        Ok(())
    }
}

async fn update_enrolled_identity(
    opts: &CommandGlobalOpts,
    node_name: &str,
) -> Result<IdentityIdentifier> {
    let identities = opts.state.identities.list()?;

    let node_state = opts.state.nodes.get(node_name)?;
    let node_identifier = node_state.config().identifier().await?;

    for mut identity in identities {
        if node_identifier == identity.config().identifier() {
            identity.set_enrollment_status()?;
        }
    }

    Ok(node_identifier)
}
