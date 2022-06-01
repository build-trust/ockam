use anyhow::anyhow;
use clap::Args;

use ockam::{Context, TcpTransport};
use ockam_multiaddr::MultiAddr;

use crate::old::identity::load_or_create_identity;
use crate::util::{embedded_node, multiaddr_to_route};
use crate::IdentityOpts;

#[derive(Clone, Debug, Args)]
pub struct EnrollCommand {
    /// Ockam's cloud address
    #[clap(display_order = 1000)]
    address: MultiAddr,

    #[clap(display_order = 1001, arg_enum, default_value = "auth0")]
    authenticator: Authenticator,

    #[clap(display_order = 1002, long, default_value = "default")]
    vault: String,

    #[clap(display_order = 1003, long, default_value = "default")]
    identity: String,

    #[clap(display_order = 1004, long)]
    overwrite: bool,
}

impl EnrollCommand {
    pub fn run(command: EnrollCommand) {
        embedded_node(enroll, command);
    }
}

async fn enroll(mut ctx: Context, command: EnrollCommand) -> anyhow::Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // TODO: The identity below will be used to create a secure channel when cloud nodes support it.
    let identity = load_or_create_identity(&IdentityOpts::from(&command), &ctx).await?;

    let route = multiaddr_to_route(&command.address)
        .ok_or_else(|| anyhow!("failed to parse address: {}", command.address))?;

    let mut api_client = ockam_api::cloud::MessagingClient::new(route, &ctx).await?;
    match command.authenticator {
        Authenticator::Auth0 => {
            api_client
                .enroll_auth0(identity.id, auth0::Auth0Service)
                .await?
        }
        Authenticator::EnrollmentToken => api_client.enroll_enrollment_token(identity.id).await?,
    }
    println!("Enrolled successfully");

    ctx.stop().await?;
    Ok(())
}

impl<'a> From<&'a EnrollCommand> for IdentityOpts {
    fn from(other: &'a EnrollCommand) -> Self {
        Self {
            overwrite: other.overwrite,
        }
    }
}

#[derive(clap::ArgEnum, Clone, Debug)]
pub enum Authenticator {
    Auth0,
    EnrollmentToken,
}

impl From<Authenticator> for ockam_api::cloud::enroll::Authenticator {
    fn from(other: Authenticator) -> Self {
        match other {
            Authenticator::Auth0 => ockam_api::cloud::enroll::Authenticator::Auth0,
            Authenticator::EnrollmentToken => {
                ockam_api::cloud::enroll::Authenticator::EnrollmentToken
            }
        }
    }
}

pub(crate) mod auth0 {
    use reqwest::StatusCode;
    use std::borrow::Borrow;
    use tokio_retry::{strategy::ExponentialBackoff, Retry};
    use tracing::{debug, warn};

    use ockam_api::cloud::enroll::auth0::*;
    use ockam_api::cloud::enroll::{Identity, TokenProvider};
    use ockam_api::error::ApiError;

    pub const DOMAIN: &str = "dev-w5hdnpc2.us.auth0.com";
    pub const CLIENT_ID: &str = "sGyXBwQfU6fjfW1gopphdV9vCLec060b";
    pub const API_AUDIENCE: &str = "https://dev-w5hdnpc2.us.auth0.com/api/v2/";
    pub const SCOPES: &str = "profile openid email";

    pub struct Auth0Service;

    #[async_trait::async_trait]
    impl<'a> TokenProvider<'a> for Auth0Service {
        type T = Auth0Token<'a>;

        async fn token(&mut self, _identity: &Identity) -> ockam_core::Result<Self::T> {
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
                .map_err(ApiError::from)?;
                match res.status() {
                    StatusCode::OK => {
                        let res = res.json::<DeviceCode>().await.map_err(ApiError::from)?;
                        debug!("device code received: {res:#?}");
                        res
                    }
                    _ => {
                        let res = res.text().await.map_err(ApiError::from)?;
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
                    .map_err(ApiError::from)?;
                match res.status() {
                    StatusCode::OK => {
                        tokens_res = res.json::<Auth0Token>().await.map_err(ApiError::from)?;
                        debug!("tokens received [tokens={tokens_res:#?}]");
                        return Ok(tokens_res);
                    }
                    _ => {
                        let err_res = res.json::<TokensError>().await.map_err(ApiError::from)?;
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
                                let err_msg =
                                    format!("failed to receive tokens [err={err_res:#?}]");
                                debug!("{}", err_msg);
                                return Err(ApiError::generic(&err_msg));
                            }
                        }
                    }
                }
            }
        }
    }
}
