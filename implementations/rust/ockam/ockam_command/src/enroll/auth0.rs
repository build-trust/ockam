use std::borrow::Borrow;
use std::io::{stdin, Write};
use std::str::FromStr;

use clap::Args;
use colorful::Colorful;
use reqwest::StatusCode;
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::{debug, info};

use ockam::identity::IdentityIdentifier;
use ockam::{Context, TcpTransport};
use ockam_api::cloud::enroll::auth0::*;
use ockam_api::cloud::project::Project;
use ockam_api::cloud::space::Space;
use ockam_api::config::cli::NodeConfig;
use ockam_api::error::ApiError;
use ockam_api::nodes::models::secure_channel::CreateSecureChannelResponse;
use ockam_core::api::Status;
use ockam_multiaddr::MultiAddr;

use crate::enroll::auth0::node::default_node;
use crate::node::NodeOpts;
use crate::secure_channel::create::SecureChannelNodeOpts;
use crate::util::api::CloudOpts;
use crate::util::output::Output;
use crate::util::{api, node_rpc, stop_node, RpcBuilder};
use crate::{exitcode, CommandGlobalOpts, EnrollCommand};

#[derive(Clone, Debug, Args)]
pub struct EnrollAuth0Command;

impl EnrollAuth0Command {
    pub fn run(opts: CommandGlobalOpts, cmd: EnrollCommand) {
        node_rpc(rpc, (opts, cmd));
    }
}

async fn rpc(ctx: Context, (opts, cmd): (CommandGlobalOpts, EnrollCommand)) -> crate::Result<()> {
    let res = run_impl(&ctx, opts, cmd).await;
    stop_node(ctx).await?;
    res
}

async fn run_impl(ctx: &Context, opts: CommandGlobalOpts, cmd: EnrollCommand) -> crate::Result<()> {
    let tcp = TcpTransport::create(ctx).await?;
    let nc = default_node(ctx, &opts, &tcp).await?;

    enroll(ctx, &opts, &cmd, &tcp, &nc).await?;

    let node_opts = NodeOpts {
        api_node: nc.name.to_string(),
    };
    let cloud_opts = cmd.cloud_opts.clone();

    let space = default_space(ctx, &opts, &tcp, &nc, &node_opts, &cloud_opts).await?;
    let project = default_project(ctx, &opts, &tcp, &nc, &space, &node_opts, &cloud_opts).await?;
    create_secure_channel_to_project(ctx, &opts, &tcp, &nc, &project, &node_opts).await?;

    Ok(())
}

// TODO: move to util/node module
mod node {
    use std::net::SocketAddr;
    use std::str::FromStr;

    use anyhow::Context;
    use tracing::{debug, trace};

    use ockam::TcpTransport;
    use ockam_api::config::cli::NodeConfig;
    use ockam_api::nodes::models::base::NodeStatus;

    use crate::node;
    use crate::util::{api, RpcBuilder};
    use crate::CommandGlobalOpts;

    pub async fn default_node(
        ctx: &ockam::Context,
        opts: &CommandGlobalOpts,
        tcp: &TcpTransport,
    ) -> crate::Result<NodeConfig> {
        let no_nodes = {
            let cfg = opts.config.get_inner();
            cfg.nodes.is_empty()
        };

        // If there are no spawned nodes, create one called "default" and return it.
        let node = if no_nodes {
            debug!("No nodes found in config, creating default node");
            create_node(opts, "default").await?
        }
        // If there are spawned nodes, return the "default" node if exists and it's running
        // or the first node we find that is running.
        else {
            let node_names = {
                let cfg = opts.config.get_inner();
                cfg.nodes
                    .iter()
                    .map(|(name, _)| name.to_string())
                    .collect::<Vec<_>>()
            };
            // Find all running nodes, skip those that are stopped.
            let mut ncs = vec![];
            for node_name in node_names.iter() {
                trace!(%node_name, "Checking node");
                let nc = opts.config.get_node(node_name)?;
                let mut rpc = RpcBuilder::new(ctx, opts, node_name).tcp(tcp).build()?;
                if rpc
                    .request_with_timeout(
                        api::node::query_status(),
                        core::time::Duration::from_millis(333),
                    )
                    .await
                    .is_err()
                {
                    trace!(%node_name, "Node is not running");
                    continue;
                }
                let ns = rpc.parse_response::<NodeStatus>()?;
                // Update PID if changed
                if nc.pid != Some(ns.pid) {
                    opts.config.update_pid(&ns.node_name, ns.pid)?;
                }
                ncs.push(nc);
            }
            // Persist PID config changes
            opts.config.atomic_update().run()?;
            // No running nodes, create a new one
            if ncs.is_empty() {
                debug!("All existing nodes are stopped, creating a new one with a random name");
                create_node(opts, None).await?
            }
            // Return the "default" node or the first one of the list
            else {
                match ncs.iter().find(|ns| ns.name == "default") {
                    None => ncs
                        .drain(..1)
                        .next()
                        .expect("already checked that is not empty"),
                    Some(n) => n.clone(),
                }
            }
        };
        debug!("Using `{}` as the default node", node.name);
        Ok(node)
    }

    async fn create_node(
        opts: &CommandGlobalOpts,
        name: impl Into<Option<&'static str>>,
    ) -> crate::Result<NodeConfig> {
        let node_name = name
            .into()
            .map(|name| name.to_string())
            .unwrap_or_else(node::random_name);
        match opts.config.select_node(&node_name) {
            Some(node) => {
                debug!(%node_name, "Returning existing node");
                Ok(node)
            }
            None => {
                debug!(%node_name, "Creating node");
                let cmd = node::CreateCommand {
                    node_name: node_name.clone(),
                    foreground: false,
                    tcp_listener_address: "127.0.0.1:0".to_string(),
                    skip_defaults: false,
                    launch_config: None,
                    no_watchdog: false,
                };
                let cmd = cmd.overwrite_addr()?;
                let addr = SocketAddr::from_str(&cmd.tcp_listener_address)
                    .context("Failed to parse tcp listener address")?;
                node::CreateCommand::create_background_node(opts, &cmd, &addr)?;
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if let Some(node) = opts.config.select_node(&node_name) {
                        return Ok(node);
                    }
                }
            }
        }
    }
}

async fn enroll(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    cmd: &EnrollCommand,
    tcp: &TcpTransport,
    nc: &NodeConfig,
) -> crate::Result<()> {
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
        Err(crate::Error::new(exitcode::SOFTWARE))
    }
}

async fn default_space<'a>(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    tcp: &TcpTransport,
    nc: &NodeConfig,
    node_opts: &NodeOpts,
    cloud_opts: &CloudOpts,
) -> crate::Result<Space<'a>> {
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
) -> crate::Result<Project<'a>> {
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
    let default_project = if available_projects.is_empty() {
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
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let mut rpc = RpcBuilder::new(ctx, opts, &nc.name).tcp(tcp).build()?;
            rpc.request(api::project::show(&cmd)).await?;
            let project = rpc.parse_response::<Project>()?;
            if project.is_ready() {
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
    println!("\n{}", default_project.output()?);
    Ok(default_project)
}

async fn create_secure_channel_to_project(
    ctx: &Context,
    opts: &CommandGlobalOpts,
    tcp: &TcpTransport,
    nc: &NodeConfig,
    project: &Project<'_>,
    node_opts: &NodeOpts,
) -> crate::Result<()> {
    // TODO: at node::service.rs, sec-channel registry should be the route, not the address
    let authorized_identifier = project.identity.clone().map(|id| {
        vec![IdentityIdentifier::from_str(id.as_ref())
            .expect("Identity received from cloud should be valid")]
    });
    let cmd = crate::secure_channel::CreateCommand {
        node_opts: SecureChannelNodeOpts {
            from: node_opts.api_node.to_string(),
        },
        addr: MultiAddr::from_str(project.access_route.as_ref()).unwrap(),
        authorized_identifier,
    };
    let mut rpc = RpcBuilder::new(ctx, opts, &nc.name).tcp(tcp).build()?;
    rpc.request(api::secure_channel::create(&cmd)).await?;
    let sc = rpc.parse_response::<CreateSecureChannelResponse>()?;
    println!("\nSecure channel to project");
    println!("  {}", sc.output()?);
    Ok(())
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
