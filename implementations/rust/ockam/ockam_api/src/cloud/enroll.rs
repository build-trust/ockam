#[cfg(test)]
use fake::{Dummy, Fake};
use minicbor::Encode;
use tracing::warn;

#[cfg(test)]
use ockam_core::compat::rand;
use ockam_core::{self, async_trait};

pub enum Authenticator {
    Auth0,
    EnrollmentToken,
}

impl core::fmt::Display for Authenticator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Authenticator::Auth0 => "Auth0".fmt(f),
            Authenticator::EnrollmentToken => "EnrollmentToken".fmt(f),
        }
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait AuthenticatorClientTrait {
    async fn auth0(&self) -> ockam_core::Result<auth0::Auth0Tokens>;
}

pub struct AuthenticatorClient;

pub(crate) mod auth0 {
    use reqwest::StatusCode;
    use tokio_retry::{strategy::ExponentialBackoff, Retry};

    use ockam_node::tokio;

    use crate::error::ApiError;

    use super::*;

    const DOMAIN: &str = "dev-w5hdnpc2.us.auth0.com";
    const CLIENT_ID: &str = "sGyXBwQfU6fjfW1gopphdV9vCLec060b";
    const API_AUDIENCE: &str = "https://dev-w5hdnpc2.us.auth0.com/api/v2/";
    const SCOPES: &str = "profile";

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct DeviceCodeResponse {
        device_code: String,
        user_code: String,
        verification_uri: String,
        verification_uri_complete: String,
        expires_in: usize,
        interval: usize,
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct TokensErrorResponse {
        error: String,
        error_description: String,
    }

    #[derive(serde::Deserialize, Encode, Debug)]
    #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
    #[cbor(map)]
    pub struct Auth0Tokens {
        #[n(0)]
        pub token_type: TokenType,
        #[n(1)]
        pub access_token: AccessToken,
    }

    #[derive(serde::Deserialize, Encode, Debug)]
    #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
    #[cbor(index_only)]
    pub enum TokenType {
        #[n(0)]
        Bearer,
    }

    #[derive(serde::Deserialize, Encode, Debug)]
    #[cfg_attr(test, derive(PartialEq, Clone, Dummy))]
    #[cbor(transparent)]
    pub struct AccessToken(#[n(0)] String);

    #[async_trait::async_trait]
    impl AuthenticatorClientTrait for AuthenticatorClient {
        async fn auth0(&self) -> ockam_core::Result<Auth0Tokens> {
            // Request device code
            // More on how to use scope and audience in https://auth0.com/docs/quickstart/native/device#device-code-parameters
            let device_code_res = {
                let retry_strategy = ExponentialBackoff::from_millis(10).take(5);
                let res = Retry::spawn(retry_strategy, move || {
                    let client = reqwest::Client::new();
                    client
                        .post(format!("https://{DOMAIN}/oauth/device/code"))
                        .header("content-type", "application/x-www-form-urlencoded")
                        .form(&[
                            ("client_id", CLIENT_ID),
                            ("scope", SCOPES),
                            ("audience", API_AUDIENCE),
                        ])
                        .send()
                })
                .await
                .map_err(ApiError::from)?;
                match res.status() {
                    StatusCode::OK => {
                        let res = res
                            .json::<DeviceCodeResponse>()
                            .await
                            .map_err(ApiError::from)?;
                        tracing::info!("device code received: {res:#?}");
                        res
                    }
                    _ => {
                        let res = res.text().await.map_err(ApiError::from)?;
                        let err = format!("couldn't get device code [response={:#?}]", res);
                        return Err(ApiError::generic(&err));
                    }
                }
            };

            // Request device activation
            // Note that we try to open the verification uri **without** the code.
            // After the code is entered, if the user closes the tab (because they
            // want to open it on another browser, for example), the uri gets
            // invalidated and the user would have to restart the process (i.e.
            // rerun the command).
            if open::that(&device_code_res.verification_uri).is_err() {
                warn!(
                    "couldn't open verification url automatically [url={}]",
                    device_code_res.verification_uri
                );
            }

            println!(
                "Open the following url in your browser to authorize your device with code {}:\n{}",
                device_code_res.user_code, device_code_res.verification_uri_complete,
            );

            // Request tokens
            let client = reqwest::Client::new();
            let tokens_res;
            loop {
                let res = client
                    .post(format!("https://{DOMAIN}/oauth/token"))
                    .header("content-type", "application/x-www-form-urlencoded")
                    .form(&[
                        ("client_id", CLIENT_ID),
                        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                        ("device_code", &device_code_res.device_code),
                    ])
                    .send()
                    .await
                    .map_err(ApiError::from)?;
                match res.status() {
                    StatusCode::OK => {
                        tokens_res = res.json::<Auth0Tokens>().await.map_err(ApiError::from)?;
                        tracing::info!("tokens received [tokes={tokens_res:#?}]");
                        return Ok(tokens_res);
                    }
                    _ => {
                        let err_res = res
                            .json::<TokensErrorResponse>()
                            .await
                            .map_err(ApiError::from)?;
                        match err_res.error.as_str() {
                            "authorization_pending" | "invalid_request" | "slow_down" => {
                                tracing::debug!("tokens not yet received [err={err_res:#?}]");
                                tokio::time::sleep(tokio::time::Duration::from_secs(
                                    device_code_res.interval as u64,
                                ))
                                .await;
                                continue;
                            }
                            _ => {
                                let err_msg =
                                    format!("failed to receive tokens [err={err_res:#?}]");
                                tracing::debug!("{}", err_msg);
                                return Err(ApiError::generic(&err_msg));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use fake::{Fake, Faker};

    use ockam_core::route;
    use ockam_node::Context;
    use ockam_transport_tcp::{TcpTransport, TCP};

    use crate::cloud::tests::TestCloudWorker;
    use crate::cloud::Client;

    use super::*;

    mod auth0 {
        use crate::cloud::enroll::auth0::Auth0Tokens;

        use super::*;

        // TODO: add tests for the auth0 internals using mockito
        // async fn internals__happy_path() {}
        // async fn internals__err_if_device_token_request_fails() {}
        // async fn internals__err_if_tokens_request_fails() {}

        #[ockam_macros::test]
        async fn happy_path(ctx: &mut Context) -> ockam_core::Result<()> {
            // Initiate cloud TCP listener
            let transport = TcpTransport::create(ctx).await?;
            let server_address = transport.listen("127.0.0.1:0").await?.to_string();
            let server_route = route![(TCP, server_address), "cloud"];
            ctx.start_worker("cloud", TestCloudWorker).await?;

            // Mock authenticator
            let expected_creds: Auth0Tokens = Faker.fake();
            let mut auth_client = MockAuthenticatorClientTrait::new();
            let moved_expected_creds = expected_creds.clone();
            auth_client
                .expect_auth0()
                .times(1)
                .return_once(move || Ok(moved_expected_creds));

            // Execute enroll
            let mut api_client = Client::new(server_route, ctx).await?;
            let _res = api_client
                .enroll(&Authenticator::Auth0, auth_client)
                .await?;

            ctx.stop().await
        }

        /// This test points to an ockam cloud to test the integration with elixir workers.
        /// The local machine must have access to cloud.ockam.io.
        #[ockam_macros::test]
        #[ignore]
        async fn cloud(ctx: &mut Context) -> ockam_core::Result<()> {
            // Mock authenticator
            let expected_creds: Auth0Tokens = Faker.fake();
            let mut auth_client = MockAuthenticatorClientTrait::new();
            let moved_expected_creds = expected_creds.clone();
            auth_client
                .expect_auth0()
                .times(1)
                .return_once(move || Ok(moved_expected_creds));

            // Enroll sending mocked tokens to cloud node
            TcpTransport::create(ctx).await?;
            let server_route = route![(TCP, "127.0.0.1:4001"), "authenticator"];
            let mut api_client = Client::new(server_route, ctx).await?;
            let _res = api_client
                .enroll(&Authenticator::Auth0, auth_client)
                .await?;

            ctx.stop().await
        }
    }
}
