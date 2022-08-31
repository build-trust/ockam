use crate::node::NodeOpts;
use crate::util::{api, connect_to, exitcode};
use crate::CommandGlobalOpts;
use anyhow::{anyhow, Context as _, Result};
use clap::{Args, Subcommand};
use minicbor::Decoder;
use ockam::Context;
use ockam_api::error::ApiError;
use ockam_api::nodes::models::services::{
    StartAuthenticatorRequest, StartCredentialsService, StartVerifierService,
};
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_core::api::{Error, Request, Response, Status};
use ockam_core::Route;
use std::path::PathBuf;
use tracing::debug;

#[derive(Clone, Debug, Args)]
pub struct StartCommand {
    #[clap(flatten)]
    pub node_opts: NodeOpts,

    #[clap(subcommand)]
    pub create_subcommand: StartSubCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum StartSubCommand {
    Vault {
        #[clap(default_value = "vault_service")]
        addr: String,
    },
    Identity {
        #[clap(default_value = "identity_service")]
        addr: String,
    },
    Authenticated {
        #[clap(default_value = "authenticated")]
        addr: String,
    },
    Verifier {
        #[clap(long, default_value = "verifier")]
        addr: String,
    },
    Credentials {
        #[clap(long, default_value = "credentials")]
        addr: String,

        #[clap(long)]
        oneway: bool,
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
    pub fn run(self, options: CommandGlobalOpts) -> Result<()> {
        let cfg = options.config;
        let port = match cfg.select_node(&self.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };

        match self.create_subcommand {
            StartSubCommand::Vault { .. } => connect_to(port, self, |mut ctx, cmd, rte| async {
                start_vault_service(&mut ctx, cmd, rte).await?;
                drop(ctx);
                Ok(())
            }),
            StartSubCommand::Identity { .. } => connect_to(port, self, |mut ctx, cmd, rte| async {
                start_identity_service(&mut ctx, cmd, rte).await?;
                drop(ctx);
                Ok(())
            }),
            StartSubCommand::Authenticated { .. } => {
                connect_to(port, self, |mut ctx, cmd, rte| async {
                    start_authenticated_service(&mut ctx, cmd, rte).await?;
                    drop(ctx);
                    Ok(())
                })
            }
            StartSubCommand::Verifier { .. } => connect_to(port, self, |mut ctx, cmd, rte| async {
                start_verifier_service(&mut ctx, cmd, rte).await?;
                drop(ctx);
                Ok(())
            }),
            StartSubCommand::Credentials { .. } => {
                connect_to(port, self, |mut ctx, cmd, rte| async {
                    start_credentials_service(&mut ctx, cmd, rte).await?;
                    drop(ctx);
                    Ok(())
                })
            }
            StartSubCommand::Authenticator { .. } => {
                connect_to(port, self, |mut ctx, cmd, rte| async {
                    start_authenticator_service(&mut ctx, cmd, rte).await?;
                    drop(ctx);
                    Ok(())
                })
            }
        }

        Ok(())
    }
}

pub async fn start_vault_service(
    ctx: &mut Context,
    cmd: StartCommand,
    mut base_route: Route,
) -> Result<()> {
    let addr = match cmd.create_subcommand {
        StartSubCommand::Vault { addr, .. } => addr,
        _ => return Err(ApiError::generic("Internal logic error").into()),
    };

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
        Ok(o) => {
            println!("{o}");
            Ok(())
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(exitcode::IOERR);
        }
    }
}

pub async fn start_identity_service(
    ctx: &mut Context,
    cmd: StartCommand,
    mut base_route: Route,
) -> Result<()> {
    let addr = match cmd.create_subcommand {
        StartSubCommand::Identity { addr, .. } => addr,
        _ => return Err(ApiError::generic("Internal logic error").into()),
    };

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
        Ok(o) => {
            println!("{o}");
            Ok(())
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(exitcode::IOERR);
        }
    }
}

pub async fn start_authenticated_service(
    ctx: &mut Context,
    cmd: StartCommand,
    mut base_route: Route,
) -> Result<()> {
    let addr = match cmd.create_subcommand {
        StartSubCommand::Authenticated { addr, .. } => addr,
        _ => return Err(ApiError::generic("Internal logic error").into()),
    };

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
        Ok(o) => {
            println!("{o}");
            Ok(())
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(exitcode::IOERR);
        }
    }
}

pub async fn start_verifier_service(
    ctx: &mut Context,
    cmd: StartCommand,
    mut route: Route,
) -> Result<()> {
    let addr = match cmd.create_subcommand {
        StartSubCommand::Verifier { addr } => addr,
        _ => unreachable!(),
    };

    let req = Request::post("/node/services/verifier")
        .body(StartVerifierService::new(&addr))
        .to_vec()?;

    let res: Vec<u8> = ctx
        .send_and_receive(route.modify().append(NODEMANAGER_ADDR), req)
        .await?;

    let mut dec = Decoder::new(&res);
    let hdr: Response = dec.decode()?;

    if let Some(Status::Ok) = hdr.status() {
        println!("Verifier service started at address: {addr}");
        return Ok(());
    }

    if hdr.has_body() {
        if let Ok(err) = dec.decode::<Error>() {
            if let Some(msg) = err.message() {
                return Err(anyhow!("Failed to start verifier service: {}", msg));
            }
        }
    }

    Err(anyhow!("Failed to start verifier service"))
}

pub async fn start_credentials_service(
    ctx: &mut Context,
    cmd: StartCommand,
    mut route: Route,
) -> Result<()> {
    let (addr, oneway) = match cmd.create_subcommand {
        StartSubCommand::Credentials { addr, oneway } => (addr, oneway),
        _ => unreachable!(),
    };

    let req = Request::post("/node/services/credentials")
        .body(StartCredentialsService::new(&addr, oneway))
        .to_vec()?;

    let res: Vec<u8> = ctx
        .send_and_receive(route.modify().append(NODEMANAGER_ADDR), req)
        .await?;

    let mut dec = Decoder::new(&res);
    let hdr: Response = dec.decode()?;

    if let Some(Status::Ok) = hdr.status() {
        println!("Credentials service started at address: {addr}");
        return Ok(());
    }

    if hdr.has_body() {
        if let Ok(err) = dec.decode::<Error>() {
            if let Some(msg) = err.message() {
                return Err(anyhow!("Failed to start credentials service: {}", msg));
            }
        }
    }

    Err(anyhow!("Failed to start credentials service"))
}

pub async fn start_authenticator_service(
    ctx: &mut Context,
    cmd: StartCommand,
    mut route: Route,
) -> Result<()> {
    let (addr, enrollers, project) = match cmd.create_subcommand {
        StartSubCommand::Authenticator {
            addr: a,
            enrollers: e,
            project: p,
        } => (a, e, p),
        _ => unreachable!(),
    };

    let req = Request::post("/node/services/authenticator")
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
        return Ok(());
    }

    if hdr.has_body() {
        if let Ok(err) = dec.decode::<Error>() {
            if let Some(msg) = err.message() {
                return Err(anyhow!("Failed to start authenticator service: {}", msg));
            }
        }
    }

    Err(anyhow!("Failed to start authenticator service"))
}
