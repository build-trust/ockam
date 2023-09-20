use std::str::FromStr;
use std::sync::Arc;

use miette::miette;
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use tiny_http::{Header, Response, Server};
use tokio::time::Duration;
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::{error, info};

use crate::cloud::enroll::auth0::{AuthorizationCode, DeviceCode, OidcToken, UserInfo};
use crate::enroll::ockam_oidc_provider::OckamOidcProvider;
use crate::enroll::oidc_provider::OidcProvider;
use crate::error::ApiError;
use ockam::compat::fmt::Debug;
use ockam_core::compat::rand::{thread_rng, RngCore};
use ockam_core::Result;
use ockam_node::callback::{new_callback, CallbackSender};
use ockam_vault::SoftwareVaultForVerifyingSignatures;

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

impl OidcService {
    /// Create an OIDC service using a specific OIDC provider
    pub fn new(provider: Arc<dyn OidcProvider + Send + Sync + 'static>) -> Self {
        Self(provider)
    }

    /// Create an OIDC service using the Ockam provider with a specific timeout for redirects
    pub fn default_with_redirect_timeout(timeout: Duration) -> Self {
        Self::new(Arc::new(OckamOidcProvider::new(timeout)))
    }

    /// Request an authorization token with a PKCE flow
    /// See the full protocol here: https://datatracker.ietf.org/doc/html/rfc7636
    pub async fn get_token_with_pkce(&self) -> Result<OidcToken> {
        let code_verifier = self.create_code_verifier();
        let authorization_code = self.authorization_code(&code_verifier).await?;
        self.retrieve_token_with_authorization_code(authorization_code, &code_verifier)
            .await
    }

    pub async fn validate_provider_config(&self) -> miette::Result<()> {
        if let Err(e) = self.device_code().await {
            return Err(miette!("Invalid OIDC configuration: {}", e));
        }
        Ok(())
    }
}

/// Implementation methods for the OidcService
impl OidcService {
    /// Return the OIDC provider
    pub fn provider(&self) -> Arc<dyn OidcProvider + Send + Sync + 'static> {
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
        let hashed = SoftwareVaultForVerifyingSignatures::compute_sha256(code_verifier.as_bytes())?;
        let code_challenge = base64_url::encode(&hashed.0);

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
            .map_err(|e| ApiError::core(e.to_string()))?;

        match res.status() {
            StatusCode::OK => {
                let res = res
                    .json::<T>()
                    .await
                    .map_err(|e| ApiError::core(e.to_string()))?;
                info!(?res, "code received: {res:#?}");
                Ok(res)
            }
            _ => {
                let res = res
                    .text()
                    .await
                    .map_err(|e| ApiError::core(e.to_string()))?;
                let err_msg = format!("couldn't get code: {:?}", res);
                error!(err_msg);
                Err(ApiError::core(err_msg))
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
        let server = Arc::new(
            Server::http(host_and_port).map_err(|_| ApiError::core("failed to set up server"))?,
        );
        info!(
            "server is started at {} and waiting for an authorization code",
            server_url
        );

        let redirect_timeout = self.provider().redirect_timeout();

        // Start a background thread which will wait for a request sending the authorization code
        tokio::task::spawn_blocking(move || {
            match server.recv_timeout(redirect_timeout) {
                Ok(Some(request)) => {
                    let code = Self::get_code(request.url())?;
                    authorization_code.send(AuthorizationCode::new(code))?;

                    // Note that a code 303 does not properly redirect to the success page
                    let response = Response::empty(302).with_header(
                        Header::from_str("Location: https://account.ockam.io/device/success")
                            .unwrap(),
                    );

                    request.respond(response).map_err(|e| {
                        ApiError::message(
                            format!("error while trying to send a response to a request on {server_url}: {e}"),
                        )
                    })
                }
                Ok(None) => Err(ApiError::message(
                    format!("timeout while trying to receive a request on {server_url} (waited for {redirect_timeout:?})"),
                )),
                Err(e) => Err(ApiError::message(
                    format!("error while trying to receive a request on {server_url}: {e}"),
                )),
            }
        });
        Ok(())
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
        let url = Url::parse(format!("http://0.0.0.0:0{}", request_url).as_str())
            .map_err(|e| ApiError::core(e.to_string()))?;

        // Check the URL path
        if !url.path().starts_with("/callback") {
            return Err(ApiError::core(format!(
                "the query path should be of the form '/callback?code=xxxx'. Got: {request_url})"
            )));
        };

        // Extract the 'code' query parameter
        if let Some((name, value)) = url.query_pairs().next() {
            if name == "code" {
                return Ok(value.to_string());
            };
        };
        Err(ApiError::core(format!(
            "could not extract the 'code' query parameter from the path {request_url})",
        )))
    }

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
            .map_err(|e| ApiError::core(e.to_string()))?;
        res.json().await.map_err(|e| ApiError::core(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "this test can only run with an open browser in order to authenticate the user"]
    async fn test_user_info() -> Result<()> {
        let oidc_service = OidcService::default_with_redirect_timeout(Duration::from_secs(15));
        let token = oidc_service.get_token_with_pkce().await?;
        let user_info = oidc_service.get_user_info(&token).await;
        assert!(user_info.is_ok());
        Ok(())
    }

    #[tokio::test]
    #[ignore = "this test can only run with an open browser in order to authenticate the user"]
    async fn test_get_token_with_pkce() -> Result<()> {
        let oidc_service = OidcService::default_with_redirect_timeout(Duration::from_secs(15));
        let token = oidc_service.get_token_with_pkce().await;
        assert!(token.is_ok());
        Ok(())
    }

    #[tokio::test]
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

    #[tokio::test]
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
}
