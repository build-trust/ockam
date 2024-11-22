use crate::enroll::oidc_provider::OidcProvider;
use ockam::identity::get_default_timeout;
use ockam_core::env::{get_env, get_env_with_default};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use std::time::Duration;
use url::Url;

const PRODUCTION_AUTHENTICATOR_ENDPOINT: &str = "https://account.ockam.io";

pub fn authenticator_endpoint() -> String {
    get_env_with_default(
        "OCKAM_AUTHENTICATOR_ENDPOINT",
        PRODUCTION_AUTHENTICATOR_ENDPOINT.to_string(),
    )
    .expect("OCKAM_AUTHENTICATOR_ENDPOINT is not valid")
    .trim_matches('/')
    .to_string()
}

/// Return the client id to use in order to access Auth0
///
pub fn auth0_client_id() -> Result<String> {
    match get_env::<String>("OCKAM_AUTH0_CLIENT_ID").ok().flatten() {
        Some(client_id) => Ok(client_id),
        None => {
            let endpoint = authenticator_endpoint();
            if endpoint == PRODUCTION_AUTHENTICATOR_ENDPOINT {
                Ok("c1SAhEjrJAqEk6ArWjGjuWX11BD2gK8X".to_string())
            } else {
                Err(Error::new(Origin::Api, Kind::NotFound, format!("The OCKAM_AUTH0_CLIENT_ID variable must be defined when using the endpoint {endpoint}")))
            }
        }
    }
}

pub struct OckamOidcProvider {
    redirect_timeout: Duration,
    base_url: String,
    client_id: String,
}

impl OckamOidcProvider {
    pub fn new() -> Result<Self> {
        OckamOidcProvider::new_with_timeout(get_default_timeout())
    }

    pub fn new_with_timeout(redirect_timeout: Duration) -> Result<Self> {
        Ok(Self {
            redirect_timeout,
            base_url: authenticator_endpoint(),
            client_id: auth0_client_id()?,
        })
    }
}

impl OidcProvider for OckamOidcProvider {
    fn client_id(&self) -> String {
        self.client_id.clone()
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
