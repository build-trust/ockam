use async_trait::async_trait;
use std::borrow::Borrow;
use std::io::stdin;

use arboard::Clipboard;
use colorful::Colorful;
use console::Term;
use miette::{miette, IntoDiagnostic};
use reqwest::StatusCode;
use tokio::time::{sleep, Duration};
use tracing::debug;

use ockam_api::cloud::enroll::auth0::*;
use ockam_api::enroll::oidc_service::OidcService;

use crate::terminal::OckamColor;
use crate::{fmt_err, fmt_log, fmt_para, CommandGlobalOpts, Result, Terminal, TerminalStream};

#[async_trait]
pub trait OidcServiceExt {
    /// Retrieve a token by having the user copy and paste a device code in their browser
    async fn get_token_interactively(&self, opts: &CommandGlobalOpts) -> Result<OidcToken>;

    /// Retrieve a token using the device code get a token from the OIDC service
    async fn get_token(&self, opts: &CommandGlobalOpts) -> Result<OidcToken>;

    async fn wait_for_email_verification(
        &self,
        token: &OidcToken,
        terminal: Option<&Terminal<TerminalStream<Term>>>,
    ) -> Result<UserInfo>;

    /// Open a browser from one of the URIs returned by a device code
    /// in order to authenticate and poll the OIDC token url to get a token back
    async fn get_token_from_browser<'a>(
        &self,
        opts: &CommandGlobalOpts,
        dc: DeviceCode<'a>,
        uri: String,
    ) -> Result<OidcToken>;

    /// Poll for an OidcToken until it's ready
    async fn poll_token<'a>(
        &'a self,
        dc: DeviceCode<'a>,
        opts: &CommandGlobalOpts,
    ) -> Result<OidcToken>;
}

#[async_trait]
impl OidcServiceExt for OidcService {
    async fn get_token_interactively(&self, opts: &CommandGlobalOpts) -> Result<OidcToken> {
        let dc = self.device_code().await?;

        // On Linux, the clipboard is cleared when the record goes out of scope, so
        // declare it up here, in the scope that bounds the entire interaction
        let mut clipboard;

        // If the terminal is quiet, write only the code at stdout so it can be processed
        if opts.terminal.is_quiet() {
            opts.terminal
                .clone()
                .stdout()
                .plain(dc.user_code.to_string())
                .write_line()?;
        }
        // Otherwise, write the instructions at stderr as normal
        else {
            clipboard = Clipboard::new();
            let otc_string = clipboard
                .as_mut()
                .ok()
                .and_then(|clip| clip.set_text(dc.user_code.to_string()).ok())
                .map_or(
                    fmt_para!(
                        "First copy this one-time code: {}",
                        format!(" {} ", dc.user_code).bg_white().black()
                    ),
                    |_| {
                        fmt_para!(
                            "Your one-time code: {} has been {} 🎉",
                            format!(" {} ", dc.user_code).bg_white().black(),
                            "copied to the clipboard".light_green()
                        )
                    },
                );

            opts.terminal
                .write_line(&fmt_log!(
                "To enroll we need to associate your Ockam identity with an Orchestrator account:\n"
            ))?
                .write_line(&otc_string)?
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
        }

        // Request device activation
        // Note that we try to open the verification uri **without** the code.
        // After the code is entered, if the user closes the tab (because they
        // want to open it on another browser, for example), the uri gets
        // invalidated and the user would have to restart the process (i.e.
        // rerun the command).
        let uri = dc.verification_uri.to_string();
        self.get_token_from_browser(opts, dc, uri).await
    }

    async fn get_token(&self, opts: &CommandGlobalOpts) -> Result<OidcToken> {
        let dc = self.device_code().await?;
        let uri = dc.verification_uri_complete.to_string();
        self.get_token_from_browser(opts, dc, uri).await
    }

    async fn wait_for_email_verification(
        &self,
        token: &OidcToken,
        terminal: Option<&Terminal<TerminalStream<Term>>>,
    ) -> Result<UserInfo> {
        let spinner_option = terminal.and_then(|t| t.progress_spinner());
        loop {
            let user_info = self.get_user_info(token).await?;
            if user_info.email_verified {
                if let Some(spinner) = spinner_option.as_ref() {
                    spinner.finish_and_clear();
                }
                terminal.map(|t| t.write_line(fmt_para!("Email <{}> verified\n", user_info.email)));
                return Ok(user_info);
            } else {
                if let Some(spinner) = spinner_option.as_ref() {
                    spinner.set_message(format!(
                        "Email <{}> pending verification. Please check your inbox...",
                        user_info.email
                    ))
                }
                sleep(Duration::from_secs(10)).await;
            }
        }
    }

    /// Open a browser from one of the URIs returned by a device code
    /// in order to authenticate and poll the OIDC token url to get a token back
    async fn get_token_from_browser<'a>(
        &self,
        opts: &CommandGlobalOpts,
        dc: DeviceCode<'a>,
        uri: String,
    ) -> Result<OidcToken> {
        if open::that(uri.clone()).is_err() {
            opts.terminal.write_line(&fmt_err!(
                "Couldn't open activation url automatically [url={}]",
                uri.light_green()
            ))?;
        }
        self.poll_token(dc, opts).await
    }

    /// Poll for an OidcToken until it's ready
    async fn poll_token<'a>(
        &'a self,
        dc: DeviceCode<'a>,
        opts: &CommandGlobalOpts,
    ) -> Result<OidcToken> {
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
                .into_diagnostic()?;
            match res.status() {
                StatusCode::OK => {
                    token = res.json::<OidcToken>().await.into_diagnostic()?;
                    debug!(?token, "token response received");
                    if let Some(spinner) = spinner_option.as_ref() {
                        spinner.finish_and_clear();
                    }
                    opts.terminal.write_line(&fmt_para!("Authenticated\n"))?;
                    return Ok(token);
                }
                _ => {
                    let err = res.json::<TokensError>().await.into_diagnostic()?;
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
}
