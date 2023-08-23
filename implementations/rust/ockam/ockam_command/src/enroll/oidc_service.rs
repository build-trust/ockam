use async_trait::async_trait;
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

use crate::enroll::oidc_provider::OidcProvider;
use crate::enroll::OckamOidcProvider;
use crate::terminal::OckamColor;
use crate::{fmt_err, fmt_log, fmt_para, CommandGlobalOpts, Result};

/// This service supports various flows of authentication with an OIDC Provider
///
/// The OidcProvider trait is currently implemented for:
///   - Ockam: uses Github and account creation with an email
///   - Okta
///
/// The main purpose of the OidcService is to authenticate a user and get
/// back an OidcToken allowing the user to connect to the Orchestrator
///
pub struct OidcService(Arc<dyn OidcProvider + Send + Sync + 'static>);

impl Default for OidcService {
    fn default() -> Self {
        OidcService::new(Arc::new(OckamOidcProvider::default()))
    }
}

#[async_trait]
pub trait GetUserInfo {
    async fn get_user_info(&self, token: &OidcToken) -> Result<UserInfo>;
}

#[async_trait]
impl GetUserInfo for OidcService {
    /// Return the information about a user once authenticated
    async fn get_user_info(&self, token: &OidcToken) -> Result<UserInfo> {
        let client = self.provider().build_http_client()?;
        let access_token = token.access_token.0.clone();
        let req = || {
            client
                .get("https://account.ockam.io/userinfo")
                .header("Authorization", format!("Bearer {}", access_token.clone()))
        };
        let retry_strategy = ExponentialBackoff::from_millis(10).take(3);
        let res = Retry::spawn(retry_strategy, move || req().send())
            .await
            .into_diagnostic()?;
        res.json().await.map_err(|e| miette!(e).into())
    }
}

pub async fn wait_for_email_verification<T: GetUserInfo>(
    client: T,
    token: &OidcToken,
    opts: &CommandGlobalOpts,
) -> Result<UserInfo> {
    let spinner_option = opts.terminal.progress_spinner();
    loop {
        let user_info = client.get_user_info(token).await?;
        if user_info.email_verified {
            if let Some(spinner) = spinner_option.as_ref() {
                spinner.finish_and_clear();
            }
            opts.terminal
                .write_line(&fmt_para!("Email <{}> verified\n", user_info.email))?;
            return Ok(user_info);
        } else {
            if let Some(spinner) = spinner_option.as_ref() {
                spinner.set_message(format!(
                    "Email <{}> pending verification. Please check your inbox...",
                    user_info.email
                ))
            }
            sleep(Duration::from_secs(5)).await;
        }
    }
}

impl OidcService {
    /// Create an OIDC service using a specific OIDC provider
    pub fn new(provider: Arc<dyn OidcProvider + Send + Sync + 'static>) -> Self {
        Self(provider)
    }

    /// Create an OIDC service using the Ockam provider with a specific timeout for redirects
    pub fn default_with_redirect_timeout(timeout: Duration) -> Self {
        Self::new(Arc::new(OckamOidcProvider::new(timeout)))
    }

    /// Retrieve a token by having the user copy and paste a device code in their browser
    pub(crate) async fn get_token_interactively(
        &self,
        opts: &CommandGlobalOpts,
    ) -> Result<OidcToken> {
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
        let uri = dc.verification_uri.to_string();
        self.get_token_from_browser(opts, dc, uri).await
    }

    /// Retrieve a token using the device code get a token from the OIDC service
    /// The device code is directly pasted to the currently opened browser window
    pub async fn get_token(&self, opts: &CommandGlobalOpts) -> Result<OidcToken> {
        let dc = self.device_code().await?;
        let uri = dc.verification_uri_complete.to_string();
        self.get_token_from_browser(opts, dc, uri).await
    }

    /// Request an authorization token with a PKCE flow
    /// See the full protocol here: https://datatracker.ietf.org/doc/html/rfc7636
    pub async fn get_token_with_pkce(&self) -> Result<OidcToken> {
        let code_verifier = self.create_code_verifier();
        let authorization_code = self.authorization_code(&code_verifier).await?;
        self.retrieve_token_with_authorization_code(authorization_code, &code_verifier)
            .await
    }

    /// Return the information about a user once authenticated
    pub async fn get_user_info(&self, token: &OidcToken) -> Result<UserInfo> {
        let client = self.provider().build_http_client()?;
        let access_token = token.access_token.0.clone();
        let req = || {
            client
                .get("https://account.ockam.io/userinfo")
                .header("Authorization", format!("Bearer {}", access_token.clone()))
        };
        let retry_strategy = ExponentialBackoff::from_millis(10).take(3);
        let res = Retry::spawn(retry_strategy, move || req().send())
            .await
            .into_diagnostic()?;
        res.json().await.map_err(|e| miette!(e).into())
    }

    pub(crate) async fn validate_provider_config(&self) -> miette::Result<()> {
        if let Err(e) = self.device_code().await {
            return Err(miette!("Invalid OIDC configuration: {}", e));
        }
        Ok(())
    }
}

/// Implementation methods for the OidcService
impl OidcService {
    /// Return the OIDC provider
    fn provider(&self) -> Arc<dyn OidcProvider + Send + Sync + 'static> {
        self.0.clone()
    }

    /// Request a device code for the current client
    pub async fn device_code(&self) -> Result<DeviceCode<'_>> {
        self.request_code(
            self.provider().device_code_url(),
            &[("scope", self.scopes())],
        )
        .await
    }

    /// Request an authorization code for the PKCE OIDC flow
    async fn authorization_code(&self, code_verifier: &str) -> Result<AuthorizationCode> {
        // Hash and base64 encode the random bytes
        // to obtain a code challenge
        let hashed = Vault::sha256(code_verifier.as_bytes());
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

    /// Retrieve a token given an authorization code obtained
    /// with a specific code verifier
    pub async fn retrieve_token_with_authorization_code(
        &self,
        authorization_code: AuthorizationCode,
        code_verifier: &str,
    ) -> Result<OidcToken> {
        info!(
            "getting an OIDC token using the authorization code {}",
            authorization_code.code
        );
        self.request_code(
            Url::parse("https://account.ockam.io/oauth/token").unwrap(),
            vec![
                ("code", authorization_code.code),
                ("code_verifier", code_verifier.to_string()),
                ("grant_type", "authorization_code".to_string()),
                ("redirect_uri", self.provider().redirect_url().to_string()),
            ]
            .as_slice(),
        )
        .await
    }

    /// Request a code from a given OIDC Provider URL
    /// This code can be a device code or an authorization code depending on the URL
    /// and the query parameters
    async fn request_code<T: DeserializeOwned + Debug>(
        &self,
        url: Url,
        query_parameters: &[(&str, String)],
    ) -> Result<T> {
        let client = self.provider().build_http_client()?;

        let parameters = {
            let mut ps = vec![("client_id", self.provider().client_id())];
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

    /// Wait for an authorization code after a redirect
    /// by starting temporarily a local web server
    /// Use the provided callback channel sender to return the authorization code asynchronously
    async fn wait_for_authorization_code(
        &self,
        authorization_code: CallbackSender<AuthorizationCode>,
    ) -> Result<()> {
        let server_url = self.provider().redirect_url();
        let host_and_port = format!(
            "{}:{}",
            server_url.host().unwrap(),
            server_url.port().unwrap()
        );
        let server = Arc::new(Server::http(host_and_port).map_err(|e| miette!(e))?);
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

                    // Note that a code 303 does not properly redirect to the success page
                    let response = Response::empty(302).with_header(
                        Header::from_str("Location: https://account.ockam.io/device/success")
                            .unwrap(),
                    );

                    request.respond(response).into_diagnostic()
                }
                Ok(None) => Err(miette!(
                    "timeout while trying to receive a request on {} (waited for {:?})",
                    server_url,
                    redirect_timeout
                )),
                Err(e) => Err(miette!(
                    "error while trying to receive a request on {}: {}",
                    server_url,
                    e
                )),
            }
        });
        Ok(())
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

    // Generate 32 random bytes as a code verifier
    // to prove that this client was really the one requesting an authorization code
    /// See the full PKCE flow here: https://datatracker.ietf.org/doc/html/rfc7636
    fn create_code_verifier(&self) -> String {
        let mut code_verifier = [0u8; 32];
        let mut rng = thread_rng();
        rng.fill_bytes(&mut code_verifier);
        base64_url::encode(&code_verifier)
    }

    /// Return the list of scopes for the authorization requests
    fn scopes(&self) -> String {
        "profile openid email".to_string()
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
    use crate::GlobalArgs;
    use ockam_api::cloud::enroll::Token;
    use ockam_node::Executor;
    use std::sync::Mutex;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "this test can only run with an open browser in order to authenticate the user"]
    async fn test_user_info() -> Result<()> {
        let oidc_service = OidcService::default_with_redirect_timeout(Duration::from_secs(15));
        let token = oidc_service.get_token_with_pkce().await?;
        let user_info = oidc_service.get_user_info(&token).await;
        assert!(user_info.is_ok());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "this test can only run with an open browser in order to authenticate the user"]
    async fn test_get_token_with_pkce() -> Result<()> {
        let oidc_service = OidcService::default_with_redirect_timeout(Duration::from_secs(15));
        let token = oidc_service.get_token_with_pkce().await;
        assert!(token.is_ok());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "this test can only run with an open browser in order to authenticate the user"]
    async fn test_authorization_code() -> Result<()> {
        let oidc_service = OidcService::default_with_redirect_timeout(Duration::from_secs(15));
        let code_verifier = oidc_service.create_code_verifier();
        let authorization_code = oidc_service
            .authorization_code(code_verifier.as_str())
            .await;
        assert!(authorization_code.is_ok());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wait_for_authorization_code() -> Result<()> {
        let oidc_service = OidcService::default();

        let (authorization_code_receiver, authorization_code_sender) = new_callback();
        oidc_service
            .wait_for_authorization_code(authorization_code_sender)
            .await?;

        let client_thread = tokio::spawn(async move {
            let client = reqwest::ClientBuilder::new().build().unwrap();
            client
                .get(oidc_service.provider().redirect_url().as_str())
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
        let code = OidcService::get_code("/callback?code=12345");
        assert!(code.is_ok());
        assert_eq!(code.unwrap(), "12345".to_string())
    }

    enum ClientState {
        FirstCall,
        SecondCall,
        Finished,
    }

    struct Client {
        state: Arc<Mutex<ClientState>>,
    }

    #[async_trait]
    impl GetUserInfo for Client {
        async fn get_user_info(&self, _token: &OidcToken) -> Result<UserInfo> {
            let mut guard = self.state.lock().unwrap();

            match *guard {
                ClientState::FirstCall => {
                    *guard = ClientState::SecondCall;
                    return Ok(UserInfo {
                        sub: "".to_string(),
                        nickname: "bad_nickname".to_string(),
                        name: "".to_string(),
                        picture: "".to_string(),
                        updated_at: "".to_string(),
                        email: "".to_string(),
                        email_verified: false,
                    });
                }

                ClientState::SecondCall => {
                    *guard = ClientState::Finished;
                    return Ok(UserInfo {
                        sub: "".to_string(),
                        nickname: "my_cool_nickname".to_string(),
                        name: "".to_string(),
                        picture: "".to_string(),
                        updated_at: "".to_string(),
                        email: "".to_string(),
                        email_verified: true,
                    });
                }

                ClientState::Finished => panic!("an extra call!"),
            }
        }
    }

    #[test]
    fn test_wait_for_email_verification() -> Result<()> {
        let opts = CommandGlobalOpts::new(GlobalArgs::default()).set_quiet();
        let authorization_code = OidcToken {
            token_type: TokenType::Bearer,
            access_token: Token("".to_string()),
        };

        let result = Executor::execute_future(async move {
            wait_for_email_verification(
                Client {
                    state: Arc::new(Mutex::new(ClientState::FirstCall)),
                },
                &authorization_code,
                &opts,
            )
            .await
        })
        .expect("TODO: panic message");

        let user_info = result.unwrap();
        assert_eq!("my_cool_nickname", user_info.nickname.as_str());
        Ok(())
    }
}
