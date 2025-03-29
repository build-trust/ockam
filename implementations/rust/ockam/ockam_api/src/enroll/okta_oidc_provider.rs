use crate::orchestrator::project::models::OktaAuth0;
use ockam_core::Result;
use std::time::Duration;
use url::Url;

use crate::enroll::oidc_provider::OidcProvider;
use crate::error::ApiError;

pub struct OktaOidcProvider {
    okta: OktaAuth0,
    redirect_timeout: Duration,
}

impl OktaOidcProvider {
    pub fn new(okta: OktaAuth0) -> Self {
        Self {
            okta,
            redirect_timeout: Duration::from_secs(120),
        }
    }
}

impl OidcProvider for OktaOidcProvider {
    fn client_id(&self) -> String {
        self.okta.client_id.clone()
    }

    fn redirect_timeout(&self) -> Duration {
        self.redirect_timeout
    }

    fn redirect_url(&self) -> Url {
        Url::parse("http://localhost:8000/callback").unwrap()
    }

    fn device_code_url(&self) -> Url {
        Url::parse(format!("{}/v1/device/authorize", &self.okta.tenant_base_url).as_str()).unwrap()
    }

    fn authorization_url(&self) -> Url {
        // See https://developer.okta.com/docs/reference/api/oidc/#composing-your-base-url
        Url::parse(format!("{}/v1/device/authorize", &self.okta.tenant_base_url).as_str()).unwrap()
    }

    fn token_request_url(&self) -> Url {
        Url::parse(format!("{}/v1/token", &self.okta.tenant_base_url).as_str()).unwrap()
    }

    fn build_http_client(&self) -> Result<reqwest::Client> {
        let certificate = reqwest::Certificate::from_pem(self.okta.certificate.as_bytes())
            .map_err(|e| ApiError::core(format!("Error parsing certificate: {}", e)))?;

        reqwest::ClientBuilder::new()
            .tls_built_in_root_certs(false)
            .add_root_certificate(certificate)
            .build()
            .map_err(|e| ApiError::core(e.to_string()))
    }
}
