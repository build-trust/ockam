use ockam_core::Result;
use std::time::Duration;
use url::Url;

/// This trait supports functionalities common to each Oidc provider
pub trait OidcProvider {
    fn client_id(&self) -> String;
    fn redirect_timeout(&self) -> Duration;
    fn redirect_url(&self) -> Url;
    fn device_code_url(&self) -> Url;
    fn authorization_url(&self) -> Url;
    fn token_request_url(&self) -> Url;
    fn build_http_client(&self) -> Result<reqwest::Client>;
}
