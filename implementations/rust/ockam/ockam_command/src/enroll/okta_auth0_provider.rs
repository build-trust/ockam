use miette::{miette, Result};

use ockam_api::cloud::project::OktaAuth0;

use crate::enroll::auth0_provider::Auth0Provider;

pub struct OktaAuth0Provider {
    okta: OktaAuth0,
}

impl OktaAuth0Provider {
    pub fn new(okta: OktaAuth0) -> Self {
        Self { okta }
    }
}

impl Auth0Provider for OktaAuth0Provider {
    fn client_id(&self) -> String {
        self.okta.client_id.clone()
    }

    fn redirect_uri(&self) -> String {
        "http://localhost:8000/callback".to_string()
    }

    fn device_code_url(&self) -> String {
        format!("{}/v1/device/authorize", &self.okta.tenant_base_url)
    }

    fn authorization_url(&self) -> String {
        // See https://developer.okta.com/docs/reference/api/oidc/#composing-your-base-url
        format!("{}/v1/device/authorize", &self.okta.tenant_base_url)
    }

    fn token_request_url(&self) -> String {
        format!("{}/v1/token", &self.okta.tenant_base_url)
    }

    fn build_http_client(&self) -> Result<reqwest::Client> {
        let certificate = reqwest::Certificate::from_pem(self.okta.certificate.as_bytes())
            .map_err(|e| miette!("Error parsing certificate: {}", e))?;

        reqwest::ClientBuilder::new()
            .tls_built_in_root_certs(false)
            .add_root_certificate(certificate)
            .build()
            .map_err(|e| miette!("Error building http client: {}", e))
    }
}
