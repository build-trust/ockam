use miette::Result;

/// This trait supports functionalities common to each Auth0 provider
pub trait Auth0Provider {
    fn client_id(&self) -> String;
    fn redirect_uri(&self) -> String;
    fn device_code_url(&self) -> String;
    fn authorization_url(&self) -> String;
    fn token_request_url(&self) -> String;
    fn build_http_client(&self) -> Result<reqwest::Client>;
}
