use std::borrow::Borrow;
use std::io::stdin;
use std::str::FromStr;
use std::sync::Arc;

use colorful::Colorful;
use miette::{miette, IntoDiagnostic};
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use tiny_http::{Header, Response, Server};
use tokio::time::{sleep, Duration};
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::{debug, error, info};

use ockam::compat::fmt::Debug;
use ockam_api::cloud::enroll::auth0::*;
use ockam_core::compat::rand::{thread_rng, RngCore};
use ockam_node::callback::{new_callback, CallbackSender};
use ockam_vault::Vault;

use crate::enroll::auth0_provider::Auth0Provider;
use crate::enroll::OckamAuth0Provider;
use crate::terminal::OckamColor;
use crate::{fmt_err, fmt_info, fmt_log, fmt_para, CommandGlobalOpts, Result};

const ENROLL_SUCCESS_RESPONSE: &str = include_str!("./static/enroll_redirect_response.txt");

pub struct Auth0Service(Arc<dyn Auth0Provider + Send + Sync + 'static>);

impl Default for Auth0Service {
    fn default() -> Self {
        Auth0Service::new(Arc::new(OckamAuth0Provider::default()))
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
    pub async fn authorization_code(&self) -> Result<AuthorizationCode> {
        // Generate 32 random bytes for the code verifier
        let code_verifier = {
            let mut code_verifier = [0u8; 32];
            let mut rng = thread_rng();
            rng.fill_bytes(&mut code_verifier);
            code_verifier
        };

        // Hash and base64 encode the random bytes
        // to obtain a code challenge
        let hashed = Vault::sha256(&code_verifier);
        let code_challenge = base64_url::encode(&hashed);

        // Start a local server to get back the authorization code after redirect
        let (authorization_code_receiver, authorization_code_sender) = new_callback();
        self.wait_for_authorization_code(authorization_code_sender)
            .await?;

        let redirect_url = self.provider().redirect_url();
        let query_parameters = vec![
            ("code_challenge_method", "S256".to_string()),
            ("response_type", "code".to_string()),
            ("code_challenge", code_challenge),
            ("redirect_uri", redirect_url.to_string()),
        ];

        let parameters = {
            let mut ps = vec![
                ("client_id", self.provider().client_id()),
                ("scope", self.scopes()),
            ];
            ps.extend_from_slice(query_parameters.as_slice());
            ps
        };

        let url = Url::parse_with_params(self.provider().authorization_url().as_str(), parameters)
            .unwrap();

        // Send a request to get an authentication code
        if open::that(url.as_str()).is_err() {
            error!(
                "Couldn't open activation url automatically [url={}]",
                url.to_string()
            );
        };

        // Wait for the authorization code to be received
        authorization_code_receiver
            .receive_timeout(self.provider().redirect_timeout())
            .await
            .map_err(|e| {
                miette!(
                    "could not retrieve an authorization code in {:?} (cause: {:?})",
                    self.provider().redirect_timeout(),
                    e
                )
                .into()
            })
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
            .into_diagnostic()?;

        match res.status() {
            StatusCode::OK => {
                let res = res.json::<T>().await.into_diagnostic()?;
                info!(?res, "code received: {res:#?}");
                Ok(res)
            }
            _ => {
                let res = res.text().await.into_diagnostic()?;
                let err_msg = format!("couldn't get code: {:?}", res);
                error!(err_msg);
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
                .into_diagnostic()?;
            match res.status() {
                StatusCode::OK => {
                    token = res.json::<Auth0Token>().await.into_diagnostic()?;
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

    pub(crate) async fn validate_provider_config(&self) -> miette::Result<()> {
        if let Err(e) = self.device_code().await {
            return Err(miette!("Invalid OIDC configuration: {}", e));
        }
        Ok(())
    }

    /// Return the list of scopes for the authorization requests
    fn scopes(&self) -> String {
        "profile openid email".to_string()
    }

    /// Wait for an authorization code after a redirect
    /// by starting temporarily a local web server
    /// Use the provided callback channel sender to return the authorization code asynchronously
    pub(crate) async fn wait_for_authorization_code(
        &self,
        authorization_code: CallbackSender<AuthorizationCode>,
    ) -> Result<()> {
        let server_url = self.provider().redirect_url();
        let host_and_port = format!(
            "{}:{}",
            server_url.host().unwrap(),
            server_url.port().unwrap()
        );
        let server = Server::http(host_and_port).map_err(|e| miette!(e))?;
        info!(
            "server is started at {} and waiting for an authorization code",
            server_url
        );

        let redirect_timeout = self.provider().redirect_timeout();

        // Start a background thread which will wait for a request sending the authorization code
        tokio::spawn(async move {
            match server.recv_timeout(redirect_timeout) {
                Ok(Some(request)) => {
                    let code = Self::get_code(request.url())?;
                    authorization_code
                        .send(AuthorizationCode::new(code))
                        .into_diagnostic()?;

                    let response = Response::from_string(ENROLL_SUCCESS_RESPONSE)
                        .with_header(Header::from_str("Content-Type: text/html").unwrap());
                    request.respond(response).into_diagnostic()
                }
                Ok(None) => Err(miette!(
                    "timeout while trying to receive a request on {} (waited for {:?})",
                    server_url,
                    redirect_timeout
                )
                .into()),
                Err(e) => Err(miette!(
                    "error while trying to receive a request on {}: {}",
                    server_url,
                    e
                )
                .into()),
            }
        });

        Ok(())
    }

    /// Extract the `code` query parameter from the callback request
    fn get_code(request_url: &str) -> Result<String> {
        // The local url is retrieved as a path associated to the tiny-http request
        // In order to parse it with the Url parser we need to first recover a full URL
        let url =
            Url::parse(format!("http://0.0.0.0:0{}", request_url).as_str()).into_diagnostic()?;

        // Check the URL path
        if !url.path().starts_with("/callback") {
            return Err(miette!(
                "the query path should be of the form '/callback?code=xxxx'. Got: {})",
                request_url
            )
            .into());
        };

        // Extract the 'code' query parameter
        if let Some((name, value)) = url.query_pairs().next() {
            if name == "code" {
                return Ok(value.to_string());
            };
        };
        Err(miette!(
            "could not extract the 'code' query parameter from the path {})",
            request_url
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore]
    async fn test_authorization_code() -> Result<()> {
        let auth0_service = Auth0Service::default();
        let authorization_code = auth0_service.authorization_code().await;
        assert!(authorization_code.is_ok());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wait_for_authorization_code() -> Result<()> {
        let auth0_service = Auth0Service::default();

        let (authorization_code_receiver, authorization_code_sender) = new_callback();
        auth0_service
            .wait_for_authorization_code(authorization_code_sender)
            .await?;

        let client_thread = tokio::spawn(async move {
            let client = reqwest::ClientBuilder::new().build().unwrap();
            client
                .get(auth0_service.provider().redirect_url().as_str())
                .query(&[("code", "12345")])
                .send()
                .await
        });

        let res = client_thread.await.unwrap();
        assert!(res.is_ok());

        let authorization_code = authorization_code_receiver
            .receive_timeout(Duration::from_secs(1))
            .await?;
        assert_eq!(authorization_code, AuthorizationCode::new("12345"));

        Ok(())
    }

    #[test]
    fn test_parse_path_query_parameters() {
        let code = Auth0Service::get_code("/callback?code=12345");
        assert!(code.is_ok());
        assert_eq!(code.unwrap(), "12345".to_string())
    }
}
