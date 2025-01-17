use std::borrow::Borrow;
use std::io::stdin;

use arboard::Clipboard;
use async_trait::async_trait;
use colorful::Colorful;
use console::Term;
use miette::{miette, IntoDiagnostic};
use reqwest::StatusCode;
use tokio::time::{sleep, Duration};
use tracing::{debug, instrument};

use ockam_api::colors::{color_email, color_uri, OckamColor};
use ockam_api::enroll::oidc_service::OidcService;
use ockam_api::orchestrator::enroll::auth0::*;
use ockam_api::terminal::{Terminal, TerminalStream};
use ockam_api::{fmt_err, fmt_log, fmt_ok};

use crate::{CommandGlobalOpts, Result};

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
    #[instrument(skip_all)]
    async fn get_token_interactively(&self, opts: &CommandGlobalOpts) -> Result<OidcToken> {
        let device_code = self.device_code().await?;

        // On Linux, the clipboard is cleared when the record goes out of scope, so
        // declare it up here, in the scope that bounds the entire interaction
        let mut clipboard;

        // If the terminal is quiet, write only the code at stdout so it can be processed
        if opts.terminal.is_quiet() {
            opts.terminal
                .clone()
                .stdout()
                .plain(device_code.user_code.to_string())
                .write_line()?;
        }
        // Otherwise, write the instructions at stderr as normal
        else {
            opts.terminal.write_line(fmt_log!(
                "Please sign into your Ockam Account to activate this machine:\n"
            ))?;

            clipboard = Clipboard::new();
            let otc_string = clipboard
                .as_mut()
                .ok()
                .and_then(|clip| clip.set_text(device_code.user_code.to_string()).ok())
                .map_or(
                    fmt_log!(
                        "You'll need to enter the one-time code: {}",
                        format!(" {} ", device_code.user_code).bg_white().black()
                    ),
                    |_| {
                        fmt_log!(
                            "You'll need to enter the one-time code: {}, {}.",
                            format!(" {} ", device_code.user_code).bg_white().black(),
                            "we've copied it to your clipboard".color(OckamColor::Success.color())
                        )
                    },
                );
            opts.terminal.write_line(&otc_string)?;

            if opts.terminal.can_ask_for_user_input() {
                opts.terminal.write(fmt_log!(
                    "Press {} to open {} in your browser.\n",
                    " ENTER ↵ ".bg_white().black().blink(),
                    color_uri(&device_code.verification_uri)
                ))?;

                let mut input = String::new();
                match stdin().read_line(&mut input) {
                    Ok(_) => {
                        opts.terminal.write_line(fmt_log!(
                            "Opening {}, in your browser, to begin activating this machine...\n",
                            color_uri(&device_code.verification_uri)
                        ))?;
                    }
                    Err(_e) => {
                        return Err(miette!(
                            "Couldn't read user input or enter keypress from stdin"
                        ))?;
                    }
                }
            } else {
                opts.terminal.write_line(fmt_log!(
                    "Open {} in your browser to begin activating this machine.\n",
                    color_uri(&device_code.verification_uri)
                ))?;
            }
        }

        // Request device activation
        // Note that we try to open the verification uri **without** the code.
        // After the code is entered, if the user closes the tab (because they
        // want to open it on another browser, for example), the uri gets
        // invalidated and the user would have to restart the process (i.e.
        // rerun the command).
        let uri = device_code.verification_uri.to_string();
        self.get_token_from_browser(opts, device_code, uri).await
    }

    async fn get_token(&self, opts: &CommandGlobalOpts) -> Result<OidcToken> {
        let dc = self.device_code().await?;
        let uri = dc.verification_uri_complete.to_string();
        self.get_token_from_browser(opts, dc, uri).await
    }

    #[instrument(skip_all)]
    async fn wait_for_email_verification(
        &self,
        token: &OidcToken,
        terminal: Option<&Terminal<TerminalStream<Term>>>,
    ) -> Result<UserInfo> {
        let pb = terminal.and_then(|t| t.spinner());
        if let Some(spinner) = pb.as_ref() {
            spinner.set_message("Verifying email...");
            sleep(Duration::from_millis(500)).await;
        }
        loop {
            let user_info = self.get_user_info(token).await?;
            if user_info.email_verified {
                if let Some(spinner) = pb.as_ref() {
                    spinner.finish_and_clear();
                }
                terminal.map(|terminal| {
                    terminal.write_line(fmt_ok!(
                        "Signed into account <{}> and activated this machine.",
                        color_email(user_info.email.to_string())
                    ))
                });
                return Ok(user_info);
            } else {
                if let Some(spinner) = pb.as_ref() {
                    spinner.set_message(format!(
                        "Email <{}> pending verification. Please check your inbox...",
                        color_email(user_info.email.to_string())
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
            opts.terminal.write_line(fmt_err!(
                "Couldn't open your browser from the terminal. Please open {} manually.",
                color_uri(&uri)
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
        let sp = opts.terminal.spinner();
        if let Some(spinner) = sp.as_ref() {
            let msg = format!(
                "{} {} {}",
                "Waiting for you to complete activating",
                "this machine".dim(),
                "using your browser..."
            );
            spinner.set_message(msg);
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
                    if let Some(spinner) = sp.as_ref() {
                        spinner.finish_and_clear();
                    }
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
                            return Err(miette!(err_msg))?;
                        }
                    }
                }
            }
        }
    }
}
