use std::borrow::Borrow;

use anyhow::anyhow;
use clap::Args;
use reqwest::StatusCode;
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::{debug, warn};

use ockam::{Context, TcpTransport};
use ockam_api::cloud::enroll::auth0::*;
use ockam_api::error::ApiError;

use crate::enroll::EnrollCommand;
use crate::old::identity::load_or_create_identity;
use crate::util::embedded_node;

#[derive(Clone, Debug, Args)]
pub struct EnrollAuth0Command;

impl EnrollAuth0Command {
    pub fn run(cmd: EnrollCommand) {
        embedded_node(enroll, cmd);
    }
}

async fn enroll(mut ctx: Context, cmd: EnrollCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let identity = load_or_create_identity(&ctx, cmd.identity_opts.overwrite).await?;

    let route = ockam_api::multiaddr_to_route(&cmd.address)
        .ok_or_else(|| anyhow!("failed to parse address: {}", cmd.address))?;

    let mut api_client = ockam_api::cloud::MessagingClient::new(route, identity, &ctx).await?;
    api_client.enroll_auth0(Auth0Service).await?;
    println!("Enrolled successfully");

    ctx.stop().await?;
    Ok(())
}

pub struct Auth0Service;

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
                    .post(format!("https://{}/oauth/device/code", DOMAIN))
                    .header("content-type", "application/x-www-form-urlencoded")
                    .form(&[
                        ("client_id", CLIENT_ID),
                        ("scope", SCOPES),
                        ("audience", API_AUDIENCE),
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

        println!(
            "Open the following url in your browser to authorize your device with code {}:\n{}",
            device_code_res.user_code, device_code_res.verification_uri_complete,
        );

        // Request tokens
        let client = reqwest::Client::new();
        let tokens_res;
        loop {
            let res = client
                .post(format!("https://{}/oauth/token", DOMAIN))
                .header("content-type", "application/x-www-form-urlencoded")
                .form(&[
                    ("client_id", CLIENT_ID),
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
