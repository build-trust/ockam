use ockam_core::Result;
use std::time::Duration;
use url::Url;

use crate::enroll::oidc_provider::OidcProvider;

pub struct OckamOidcProvider {
    redirect_timeout: Duration,
}

impl Default for OckamOidcProvider {
    fn default() -> Self {
        OckamOidcProvider::new(Duration::from_secs(120))
    }
}

impl OckamOidcProvider {
    pub fn new(redirect_timeout: Duration) -> Self {
        Self { redirect_timeout }
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
        Url::parse("https://account.ockam.io/oauth/device/code").unwrap()
    }

    fn authorization_url(&self) -> Url {
        Url::parse("https://account.ockam.io/authorize").unwrap()
    }

    fn token_request_url(&self) -> Url {
        Url::parse("https://account.ockam.io/oauth/token").unwrap()
    }

    fn build_http_client(&self) -> Result<reqwest::Client> {
        Ok(reqwest::Client::new())
    }
}
