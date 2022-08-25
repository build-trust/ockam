use anyhow::anyhow;
use std::borrow::Borrow;
use std::io::{stdin, Write};

use clap::Args;
use colorful::Colorful;
use reqwest::StatusCode;
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::{debug, info};

use ockam::{Context, TcpTransport};
use ockam_api::cloud::enroll::auth0::*;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::space::Space;
use ockam_api::config::cli::NodeConfig;
use ockam_api::error::ApiError;
use ockam_core::api::Status;

use crate::node::NodeOpts;
use crate::util::api::CloudOpts;
use crate::util::node::default_node;
use crate::util::output::Output;
use crate::util::{api, node_rpc, stop_node, RpcBuilder};
use crate::{CommandGlobalOpts, EnrollCommand, Result};

#[derive(Clone, Debug, Args)]
pub struct EnrollAuth0Command;

impl EnrollAuth0Command {
    pub fn run(opts: CommandGlobalOpts, cmd: EnrollCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, EnrollCommand)) -> Result<()> {
    let res = run_impl(&ctx, opts, cmd).await;
    stop_node(ctx).await?;
    res
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: EnrollCommand) -> Result<()> {
    let tcp = TcpTransport::create(ctx).await?;
    let nc = default_node(ctx, &opts, &tcp).await?;

    enroll(ctx, &opts, &cmd, &tcp, &nc).await?;

    let node_opts = NodeOpts {
        api_node: nc.name.to_string(),
    };
    let cloud_opts = cmd.cloud_opts.clone();

    let space = default_space(ctx, &opts, &tcp, &nc, &node_opts, &cloud_opts).await?;
    default_project(ctx, &opts, &tcp, &nc, &space, &node_opts, &cloud_opts).await?;

    Ok(())
}

async fn enroll(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    cmd: &EnrollCommand,
    tcp: &TcpTransport,
    nc: &NodeConfig,
) -> anyhow::Result<()> {
    let auth0 = Auth0Service;
    let token = auth0.token().await?;
    let mut rpc = RpcBuilder::new(ctx, opts, &nc.name).tcp(tcp).build()?;
    rpc.request(api::enroll::auth0(cmd.clone(), token)).await?;
    let (res, dec) = rpc.check_response()?;
    if res.status() == Some(Status::Ok) {
        info!("Enrolled successfully");
        Ok(())
    } else if res.status() == Some(Status::BadRequest) {
        info!("Already enrolled");
        Ok(())
    } else {
        eprintln!("{}", rpc.parse_err_msg(res, dec));
        Err(anyhow!("Failed to enroll"))
    }
}

async fn default_space<'a>(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    tcp: &TcpTransport,
    nc: &NodeConfig,
    node_opts: &NodeOpts,
    cloud_opts: &CloudOpts,
) -> Result<Space<'a>> {
    // Get available spaces for node's identity
    let mut rpc = RpcBuilder::new(ctx, opts, &nc.name).tcp(tcp).build()?;
    let mut available_spaces = {
        let cmd = crate::space::ListCommand {
            node_opts: node_opts.clone(),
            cloud_opts: cloud_opts.clone(),
        };
        rpc.request(api::space::list(&cmd)).await?;
        rpc.parse_response::<Vec<Space>>()?
    };
    // If the identity has no spaces, create one
    let default_space = if available_spaces.is_empty() {
        let cmd = crate::space::CreateCommand {
            node_opts: node_opts.clone(),
            cloud_opts: cloud_opts.clone(),
            name: crate::space::random_name(),
            admins: vec![],
        };
        let mut rpc = RpcBuilder::new(ctx, opts, &nc.name).tcp(tcp).build()?;
        rpc.request(api::space::create(&cmd)).await?;
        rpc.parse_response::<Space>()?.to_owned()
    }
    // If it has, return the first one on the list
    else {
        available_spaces
            .drain(..1)
            .next()
            .expect("already checked that is not empty")
            .to_owned()
    };
    println!("\n{}", default_space.output()?);
    Ok(default_space)
}

async fn default_project<'a>(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    tcp: &TcpTransport,
    nc: &NodeConfig,
    space: &Space<'_>,
    node_opts: &NodeOpts,
    cloud_opts: &CloudOpts,
) -> Result<Project<'a>> {
    // Get available project for the given space
    let mut rpc = RpcBuilder::new(ctx, opts, &nc.name).tcp(tcp).build()?;
    let mut available_projects: Vec<Project> = {
        let cmd = crate::project::ListCommand {
            node_opts: node_opts.clone(),
            cloud_opts: cloud_opts.clone(),
        };
        rpc.request(api::project::list(&cmd)).await?;
        rpc.parse_response::<Vec<Project>>()?
    };
    // If the space has no projects, create one
    let mut default_project = if available_projects.is_empty() {
        let cmd = crate::project::CreateCommand {
            space_id: space.id.to_string(),
            project_name: "default".to_string(),
            node_opts: node_opts.clone(),
            cloud_opts: cloud_opts.clone(),
            services: vec![], // TODO: define default services
        };
        let mut rpc = RpcBuilder::new(ctx, opts, &nc.name).tcp(tcp).build()?;
        rpc.request(api::project::create(&cmd)).await?;
        rpc.parse_response::<Project>()?.to_owned()
    }
    // If it has, return the "default" project or first one on the list
    else {
        match available_projects.iter().find(|ns| ns.name == "default") {
            None => available_projects
                .drain(..1)
                .next()
                .expect("already checked that is not empty")
                .to_owned(),
            Some(p) => p.to_owned(),
        }
    };

    if default_project.access_route.is_empty() {
        print!("\nProject created. Waiting until it's operative...");
        let cmd = crate::project::ShowCommand {
            space_id: space.id.to_string(),
            project_id: default_project.id.to_string(),
            node_opts: node_opts.clone(),
            cloud_opts: cloud_opts.clone(),
        };
        loop {
            print!(".");
            std::io::stdout().flush()?;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let mut rpc = RpcBuilder::new(ctx, opts, &nc.name).tcp(tcp).build()?;
            rpc.request(api::project::show(&cmd)).await?;
            let project = rpc.parse_response::<Project>()?;
            if project.is_ready() {
                default_project = project.to_owned();
                break;
            }
        }
    }

    // Store the default project in the config lookup table.
    opts.config.set_project_alias(
        default_project.name.to_string(),
        default_project.access_route.to_string(),
        default_project.id.to_string(),
        default_project
            .identity
            .as_ref()
            .expect("Project should have identity set")
            .to_string(),
    )?;
    opts.config.atomic_update().run()?;
    println!("\n{}", default_project.output()?);
    Ok(default_project)
}

pub struct Auth0Service;

impl Auth0Service {
    const DOMAIN: &'static str = "account.ockam.io";
    const CLIENT_ID: &'static str = "c1SAhEjrJAqEk6ArWjGjuWX11BD2gK8X";
    const SCOPES: &'static str = "profile openid email";
}

#[async_trait::async_trait]
impl Auth0TokenProvider for Auth0Service {
    async fn token(&self) -> ockam_core::Result<Auth0Token> {
        // Request device code
        // More on how to use scope and audience in https://auth0.com/docs/quickstart/native/device#device-code-parameters
        let device_code_res = {
            let retry_strategy = ExponentialBackoff::from_millis(10).take(5);
            let res = Retry::spawn(retry_strategy, move || {
                let client = reqwest::Client::new();
                client
                    .post(format!("https://{}/oauth/device/code", Self::DOMAIN))
                    .header("content-type", "application/x-www-form-urlencoded")
                    .form(&[("client_id", Self::CLIENT_ID), ("scope", Self::SCOPES)])
                    .send()
            })
            .await
            .map_err(|err| ApiError::generic(&err.to_string()))?;
            match res.status() {
                StatusCode::OK => {
                    let res = res
                        .json::<DeviceCode>()
                        .await
                        .map_err(|err| ApiError::generic(&err.to_string()))?;
                    debug!("device code received: {res:#?}");
                    res
                }
                _ => {
                    let res = res
                        .text()
                        .await
                        .map_err(|err| ApiError::generic(&err.to_string()))?;
                    let err = format!("couldn't get device code [response={:#?}]", res);
                    return Err(ApiError::generic(&err));
                }
            }
        };

        eprint!(
            "\nEnroll Ockam Command's default identity with Ockam Orchestrator:\n\
             {} First copy your one-time code: {}\n\
             {} Then press enter to open {} in your browser...",
            "!".light_yellow(),
            format!(" {} ", device_code_res.user_code)
                .bg_white()
                .black(),
            ">".light_green(),
            device_code_res.verification_uri.to_string().light_green(),
        );

        let mut input = String::new();
        match stdin().read_line(&mut input) {
            Ok(_) => eprintln!(
                "{} Opening: {}",
                ">".light_green(),
                device_code_res.verification_uri
            ),
            Err(_e) => {
                return Err(ApiError::generic("couldn't read enter from stdin"));
            }
        }

        // Request device activation
        // Note that we try to open the verification uri **without** the code.
        // After the code is entered, if the user closes the tab (because they
        // want to open it on another browser, for example), the uri gets
        // invalidated and the user would have to restart the process (i.e.
        // rerun the command).
        let uri: &str = device_code_res.verification_uri.borrow();
        if open::that(uri).is_err() {
            eprintln!(
                "{} Couldn't open activation url automatically [url={}]",
                "!".light_red(),
                uri.to_string().light_green()
            );
        }

        // Request tokens
        let client = reqwest::Client::new();
        let tokens_res;
        loop {
            let res = client
                .post(format!("https://{}/oauth/token", Self::DOMAIN))
                .header("content-type", "application/x-www-form-urlencoded")
                .form(&[
                    ("client_id", Self::CLIENT_ID),
                    ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ("device_code", &device_code_res.device_code),
                ])
                .send()
                .await
                .map_err(|err| ApiError::generic(&err.to_string()))?;
            match res.status() {
                StatusCode::OK => {
                    tokens_res = res
                        .json::<Auth0Token>()
                        .await
                        .map_err(|err| ApiError::generic(&err.to_string()))?;
                    debug!("tokens received [tokens={tokens_res:#?}]");
                    eprintln!("{} Tokens received, processing...", ">".light_green());
                    return Ok(tokens_res);
                }
                _ => {
                    let err_res = res
                        .json::<TokensError>()
                        .await
                        .map_err(|err| ApiError::generic(&err.to_string()))?;
                    match err_res.error.borrow() {
                        "authorization_pending" | "invalid_request" | "slow_down" => {
                            debug!("tokens not yet received [err={err_res:#?}]");
                            tokio::time::sleep(tokio::time::Duration::from_secs(
                                device_code_res.interval as u64,
                            ))
                            .await;
                            continue;
                        }
                        _ => {
                            let err_msg = format!("failed to receive tokens [err={err_res:#?}]");
                            debug!("{}", err_msg);
                            return Err(ApiError::generic(&err_msg));
                        }
                    }
                }
            }
        }
    }
}
