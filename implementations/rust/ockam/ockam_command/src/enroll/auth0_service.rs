use std::borrow::Borrow;
use std::io::stdin;
use std::sync::Arc;

use colorful::Colorful;
use miette::miette;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use tokio::time::{sleep, Duration};
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::debug;

use ockam::compat::fmt::Debug;
use ockam_api::cloud::enroll::auth0::*;

use crate::enroll::auth0_provider::Auth0Provider;
use crate::enroll::OckamAuth0Provider;
use crate::terminal::OckamColor;
use crate::{fmt_err, fmt_info, fmt_log, fmt_para, CommandGlobalOpts, Result};

pub struct Auth0Service(Arc<dyn Auth0Provider + Send + Sync + 'static>);

impl Default for Auth0Service {
    fn default() -> Self {
        Auth0Service::new(Arc::new(OckamAuth0Provider {}))
    }
}

impl Auth0Service {
    pub fn new(provider: Arc<dyn Auth0Provider + Send + Sync + 'static>) -> Self {
        Self(provider)
    }

    fn provider(&self) -> Arc<dyn Auth0Provider + Send + Sync + 'static> {
        self.0.clone()
    }

    pub(crate) async fn get_token_interactively(
        &self,
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

    /// Using the device code get a token from the Auth0 service
    /// The device code is directly pasted to the current opened browser window
    pub async fn get_token(&self, opts: &CommandGlobalOpts) -> Result<Auth0Token> {
        let dc = self.device_code().await?;

        let uri: &str = &dc.verification_uri_complete;
        opts.terminal
            .write_line(&fmt_info!("Opening {}", uri.to_string().light_green()))?;
        if open::that(uri).is_err() {
            opts.terminal.write_line(&fmt_err!(
                "Couldn't open activation url automatically [url={}]",
                uri.to_string().light_green()
            ))?;
        }
        self.poll_token(dc, opts).await
    }

    /// Request a device code
    pub async fn device_code(&self) -> Result<DeviceCode<'_>> {
        self.request_code(self.provider().device_code_url(), &[])
            .await
    }

    /// Request an authorization code
    /// » echo "https://account.ockam.io/authorize?client_id=${CLIENT_ID}&code_challenge=${CODE_CHALLENGE}&code_challenge_method=S256&response_type=code&redirect_uri=${REDIRECT_URI}&scope=${SCOPE}"
    pub async fn authorization_code(&self) -> Result<AuthorizationCode> {
        let code_challenge = "";
        let query_parameters = vec![
            ("code_channel_method", "S256".to_string()),
            ("response_type", "code".to_string()),
            ("code_challenge", code_challenge.to_string()),
            ("redirect_uri", self.provider().redirect_uri()),
        ];
        self.request_code(
            self.provider().authorization_url(),
            query_parameters.as_slice(),
        )
        .await
    }

    /// Request a code from a given Auth0 URL
    async fn request_code<T: DeserializeOwned + Debug>(
        &self,
        url: String,
        query_parameters: &[(&str, String)],
    ) -> Result<T> {
        let client = self.provider().build_http_client()?;

        let parameters = {
            let mut ps = vec![
                ("client_id", self.provider().client_id()),
                ("scope", self.scopes()),
            ];
            ps.extend_from_slice(query_parameters);
            ps
        };

        let req = || {
            client
                .post(url.clone())
                .header("content-type", "application/x-www-form-urlencoded")
                .form(&parameters)
        };
        let retry_strategy = ExponentialBackoff::from_millis(10).take(3);
        let res = Retry::spawn(retry_strategy, move || req().send())
            .await
            .map_err(|e| miette!(e.to_string()))?;

        match res.status() {
            StatusCode::OK => {
                let res = res.json::<T>().await.map_err(|e| miette!(e.to_string()))?;
                debug!(?res, "code received: {res:#?}");
                Ok(res)
            }
            _ => {
                let res = res.text().await.map_err(|e| miette!(e.to_string()))?;
                let err_msg = "couldn't get code";
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
        let provider = self.provider();
        let client = provider.build_http_client()?;
        let token;
        let spinner_option = opts.terminal.progress_spinner();
        if let Some(spinner) = spinner_option.as_ref() {
            spinner.set_message("Waiting for you to complete authentication using your browser...");
        }
        loop {
            let res = client
                .post(provider.token_request_url())
                .header("content-type", "application/x-www-form-urlencoded")
                .form(&[
                    ("client_id", self.provider().client_id()),
                    (
                        "grant_type",
                        "urn:ietf:params:oauth:grant-type:device_code".to_string(),
                    ),
                    ("device_code", dc.device_code.to_string()),
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

    fn scopes(&self) -> String {
        "profile openid email".to_string()
    }
}
