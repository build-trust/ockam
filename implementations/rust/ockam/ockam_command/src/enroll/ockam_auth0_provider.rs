use crate::enroll::auth0_provider::Auth0Provider;
use miette::Result;

pub struct OckamAuth0Provider {}

impl Auth0Provider for OckamAuth0Provider {
    fn client_id(&self) -> String {
        "c1SAhEjrJAqEk6ArWjGjuWX11BD2gK8X".to_string()
    }

    fn redirect_uri(&self) -> String {
        "http://localhost:8000/callback".to_string()
    }

    fn device_code_url(&self) -> String {
        "https://account.ockam.io/oauth/device/code".to_string()
    }

    fn authorization_url(&self) -> String {
        "https://account.ockam.io/authorize".to_string()
    }

    fn token_request_url(&self) -> String {
        "https://account.ockam.io/oauth/token".to_string()
    }

    fn build_http_client(&self) -> Result<reqwest::Client> {
        Ok(reqwest::Client::new())
    }
}
