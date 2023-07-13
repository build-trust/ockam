use miette::Result;
use std::time::Duration;
use url::Url;

use crate::enroll::auth0_provider::Auth0Provider;

pub struct OckamAuth0Provider {
    redirect_timeout: Duration,
}

impl Default for OckamAuth0Provider {
    fn default() -> Self {
        OckamAuth0Provider::new(Duration::from_secs(60))
    }
}

impl OckamAuth0Provider {
    pub fn new(redirect_timeout: Duration) -> Self {
        Self { redirect_timeout }
    }
}

impl Auth0Provider for OckamAuth0Provider {
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
