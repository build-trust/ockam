use anyhow::{anyhow, Context};
use clap::Args;
use minicbor::Decoder;
use reqwest::StatusCode;
use std::borrow::Borrow;
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::{debug, warn};

use ockam_api::cloud::enroll::auth0::*;
use ockam_api::error::ApiError;
use ockam_api::nodes::NODEMANAGER_ADDR;
use ockam_api::{Response, Status};
use ockam_core::Route;

use crate::util::{api, connect_to, exitcode, stop_node};
use crate::{CommandGlobalOpts, EnrollCommand};

#[derive(Clone, Debug, Args)]
pub struct EnrollAuth0Command;

impl EnrollAuth0Command {
    pub fn run(opts: CommandGlobalOpts, cmd: EnrollCommand) {
        let cfg = &opts.config;
        let port = match cfg.select_node(&cmd.node_opts.api_node) {
            Some(cfg) => cfg.port,
            None => {
                eprintln!("No such node available.  Run `ockam node list` to list available nodes");
                std::process::exit(exitcode::IOERR);
            }
        };
        connect_to(port, (opts, cmd), enroll);
    }
}

async fn enroll(
    ctx: ockam::Context,
    (_opts, cmd): (CommandGlobalOpts, EnrollCommand),
    mut base_route: Route,
) -> anyhow::Result<()> {
    let auth0 = Auth0Service;
    let token = auth0.token().await?;

    let route: Route = base_route.modify().append(NODEMANAGER_ADDR).into();
    debug!(?cmd, %route, "Sending request");

    let response: Vec<u8> = ctx
        .send_and_receive(route, api::enroll::auth0(cmd, token)?)
        .await
        .context("Failed to process request")?;
    let mut dec = Decoder::new(&response);
    let header = dec.decode::<Response>()?;
    debug!(?header, "Received response");

    let res = match header.status() {
        Some(Status::Ok) => {
            let output = "Enrolled successfully".to_string();
            Ok(output)
        }
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

pub struct Auth0Service;

impl Auth0Service {
    const DOMAIN: &'static str = "dev-w5hdnpc2.us.auth0.com";
    const CLIENT_ID: &'static str = "sGyXBwQfU6fjfW1gopphdV9vCLec060b";
    const API_AUDIENCE: &'static str = "https://dev-w5hdnpc2.us.auth0.com/api/v2/";
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
                    .form(&[
                        ("client_id", Self::CLIENT_ID),
                        ("scope", Self::SCOPES),
                        ("audience", Self::API_AUDIENCE),
                    ])
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

        // Request device activation
        // Note that we try to open the verification uri **without** the code.
        // After the code is entered, if the user closes the tab (because they
        // want to open it on another browser, for example), the uri gets
        // invalidated and the user would have to restart the process (i.e.
        // rerun the command).
        let uri: &str = device_code_res.verification_uri.borrow();
        if open::that(uri).is_err() {
            warn!("couldn't open verification url automatically [url={uri}]",);
        }

        eprintln!(
            "Open the following url in your browser to authorize your device with code {}:\n{}",
            device_code_res.user_code, device_code_res.verification_uri_complete,
        );

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
                    eprintln!("Tokens received, processing...",);
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
