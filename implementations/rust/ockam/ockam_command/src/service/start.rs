use std::path::PathBuf;

use crate::node::NodeOpts;
use crate::util::{api, connect_to, exitcode, stop_node};
use crate::CommandGlobalOpts;
use anyhow::{anyhow, Context};
use clap::{Args, Subcommand};
use minicbor::Decoder;
use ockam_api::error::ApiError;
use ockam_api::nodes::models::services::{StartAuthenticatorRequest, StartVerifierService};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{Error, Method, Request, Response, Status};
use ockam_core::Route;
use tracing::debug;

#[derive(Clone, Debug, Args)]
pub struct StartCommand {
    #[clap(flatten)]
    node_opts: NodeOpts,

    #[clap(subcommand)]
    pub create_subcommand: StartSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum StartSubCommand {
    Vault {
        addr: Option<String>,
    },
    Identity {
        addr: Option<String>,
    },
    Authenticated {
        addr: Option<String>,
    },
    Verifier {
        #[clap(long, default_value = "verifier")]
        addr: String,
    },
    Authenticator {
        #[clap(long, default_value = "authenticator")]
        addr: String,

        #[clap(long)]
        enrollers: PathBuf,

        #[clap(long)]
        project: String,
    },
}

impl StartCommand {
    pub fn run(opts: CommandGlobalOpts, command: StartCommand) -> anyhow::Result<()> {
        let cfg = opts.config;
        let port = match cfg.select_node(&command.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };

        match command.create_subcommand {
            StartSubCommand::Vault { .. } => connect_to(port, command, start_vault_service),
            StartSubCommand::Identity { .. } => connect_to(port, command, start_identity_service),
            StartSubCommand::Authenticated { .. } => {
                connect_to(port, command, start_authenticated_service)
            }
            StartSubCommand::Verifier { .. } => connect_to(port, command, start_verifier_service),
            StartSubCommand::Authenticator { .. } => {
                connect_to(port, command, start_authenticator_service)
            }
        }

        Ok(())
    }
}

pub async fn start_vault_service(
    ctx: ockam::Context,
    cmd: StartCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let addr = match cmd.create_subcommand {
        StartSubCommand::Vault { addr, .. } => addr,
        _ => return Err(ApiError::generic("Internal logic error").into()),
    };

    let addr = addr.unwrap_or_else(|| "vault_service".to_string());

    let response: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::start_vault_service(&addr)?,
        )
        .await
        .context("Failed to process request")?;

    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => Ok(format!(
            "Vault Service started! You can send messages to it via this address:\n{}",
            addr
        )),
        Some(Status::InternalServerError) => {
            let err = dec
                .decode::<String>()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!(
                "An error occurred while processing the request: {err}"
            ))
        }
        _ => Err(anyhow!("Unexpected response received from node")),
    };
    match res {
        Ok(o) => println!("{o}"),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(exitcode::IOERR);
        }
    };

    stop_node(ctx).await
}

pub async fn start_identity_service(
    ctx: ockam::Context,
    cmd: StartCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let addr = match cmd.create_subcommand {
        StartSubCommand::Identity { addr, .. } => addr,
        _ => return Err(ApiError::generic("Internal logic error").into()),
    };

    let addr = addr.unwrap_or_else(|| "identity_service".to_string());

    let response: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::start_identity_service(&addr)?,
        )
        .await
        .context("Failed to process request")?;

    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => Ok(format!(
            "Identity Service started! You can send messages to it via this address:\n{}",
            addr
        )),
        Some(Status::InternalServerError) => {
            let err = dec
                .decode::<String>()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!(
                "An error occurred while processing the request: {err}"
            ))
        }
        _ => Err(anyhow!("Unexpected response received from node")),
    };
    match res {
        Ok(o) => println!("{o}"),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(exitcode::IOERR);
        }
    };

    stop_node(ctx).await
}

pub async fn start_authenticated_service(
    ctx: ockam::Context,
    cmd: StartCommand,
    mut base_route: Route,
) -> anyhow::Result<()> {
    let addr = match cmd.create_subcommand {
        StartSubCommand::Authenticated { addr, .. } => addr,
        _ => return Err(ApiError::generic("Internal logic error").into()),
    };

    let addr = addr.unwrap_or_else(|| "authenticated".to_string());

    let response: Vec<u8> = ctx
        .send_and_receive(
            base_route.modify().append(NODEMANAGER_ADDR),
            api::start_authenticated_service(&addr)?,
        )
        .await
        .context("Failed to process request")?;

    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => Ok(format!(
            "Authenticated Service started! You can send messages to it via this address:\n{}",
            addr
        )),
        Some(Status::InternalServerError) => {
            let err = dec
                .decode::<String>()
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow!(
                "An error occurred while processing the request: {err}"
            ))
        }
        _ => Err(anyhow!("Unexpected response received from node")),
    };
    match res {
        Ok(o) => println!("{o}"),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(exitcode::IOERR);
        }
    };

    stop_node(ctx).await
}

pub async fn start_verifier_service(
    ctx: ockam::Context,
    cmd: StartCommand,
    mut route: Route,
) -> anyhow::Result<()> {
    let addr = match cmd.create_subcommand {
        StartSubCommand::Verifier { addr } => addr,
        _ => unreachable!(),
    };

    let req = Request::builder(Method::Post, "/node/services/verifier")
        .body(StartVerifierService::new(&addr))
        .to_vec()?;

    let res: Vec<u8> = ctx
        .send_and_receive(route.modify().append(NODEMANAGER_ADDR), req)
        .await?;

    let mut dec = Decoder::new(&res);
    let hdr: Response = dec.decode()?;

    if let Some(Status::Ok) = hdr.status() {
        println!("Verifier service started at address: {addr}");
        return stop_node(ctx).await;
    }

    if hdr.has_body() {
        if let Ok(err) = dec.decode::<Error>() {
            if let Some(msg) = err.message() {
                eprintln!("Failed to start verifier service: {}", msg);
                return stop_node(ctx).await;
            }
        }
    }

    eprintln!("Failed to start verifier service");
    stop_node(ctx).await
}

pub async fn start_authenticator_service(
    ctx: ockam::Context,
    cmd: StartCommand,
    mut route: Route,
) -> anyhow::Result<()> {
    let (addr, enrollers, project) = match cmd.create_subcommand {
        StartSubCommand::Authenticator {
            addr: a,
            enrollers: e,
            project: p,
        } => (a, e, p),
        _ => unreachable!(),
    };

    let req = Request::builder(Method::Post, "/node/services/authenticator")
        .body(StartAuthenticatorRequest::new(
            &addr,
            &enrollers,
            project.as_bytes(),
        ))
        .to_vec()?;

    let res: Vec<u8> = ctx
        .send_and_receive(route.modify().append(NODEMANAGER_ADDR), req)
        .await?;

    let mut dec = Decoder::new(&res);
    let hdr: Response = dec.decode()?;

    if let Some(Status::Ok) = hdr.status() {
        println!("Authenticator service started at address: {addr}");
        return stop_node(ctx).await;
    }

    if hdr.has_body() {
        if let Ok(err) = dec.decode::<Error>() {
            if let Some(msg) = err.message() {
                eprintln!("Failed to start authenticator service: {}", msg);
                return stop_node(ctx).await;
            }
        }
    }

    eprintln!("Failed to start authenticator service");
    stop_node(ctx).await
}
