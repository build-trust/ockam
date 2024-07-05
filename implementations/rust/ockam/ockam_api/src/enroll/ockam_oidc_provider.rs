use ockam::identity::get_default_timeout;
use ockam_core::env::get_env_with_default;
use ockam_core::Result;
use std::time::Duration;
use url::Url;

use crate::enroll::oidc_provider::OidcProvider;

pub fn authenticator_endpoint() -> String {
    get_env_with_default(
        "OCKAM_AUTHENTICATOR_ENDPOINT",
        "https://account.ockam.io".to_string(),
    )
    .expect("OCKAM_AUTHENTICATOR_ENDPOINT is not valid")
    .trim_matches('/')
    .to_string()
}

pub struct OckamOidcProvider {
    redirect_timeout: Duration,
    base_url: String,
}

impl Default for OckamOidcProvider {
    fn default() -> Self {
        OckamOidcProvider::new(get_default_timeout())
    }
}

impl OckamOidcProvider {
    pub fn new(redirect_timeout: Duration) -> Self {
        Self {
            redirect_timeout,
            base_url: authenticator_endpoint(),
        }
    }
}

impl OidcProvider for OckamOidcProvider {
    fn client_id(&self) -> String {
        "c1SAhEjrJAqEk6ArWjGjuWX11BD2gK8X".to_string()
    }

    fn redirect_timeout(&self) -> Duration {
        self.redirect_timeout
    }

    fn redirect_url(&self) -> Url {
        Url::parse("http://localhost:8000/callback").unwrap()
    }

    fn device_code_url(&self) -> Url {
        Url::parse(&format!("{}/oauth/device/code", self.base_url)).unwrap()
    }

    fn authorization_url(&self) -> Url {
        Url::parse(&format!("{}/authorize", self.base_url)).unwrap()
    }

    fn token_request_url(&self) -> Url {
        Url::parse(&format!("{}/oauth/token", self.base_url)).unwrap()
    }

    fn build_http_client(&self) -> Result<reqwest::Client> {
        Ok(reqwest::Client::new())
    }
}
