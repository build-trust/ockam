use anyhow::anyhow;
use clap::Args;

use crate::api::{self, CloudApi, NodeCloudApi, RequestMethod};
use crate::identity::load_or_create_identity_and_vault;
use crate::{storage, IdentityOpts};

pub async fn run(args: EnrollCommandArgs, mut ctx: ockam::Context) -> anyhow::Result<()> {
    // TODO: The identity created below will be later used to create a secure channel.
    let ockam_dir = storage::init_ockam_dir()?;
    let (_identity, _vault) =
        load_or_create_identity_and_vault(&IdentityOpts::from(&args), &ctx, &ockam_dir).await?;
    let api_client = NodeCloudApi::from(&mut ctx);
    let auth_client = AuthenticatorClient;
    authenticate(&args.authenticator, auth_client, api_client).await?;
    ctx.stop().await?;
    Ok(())
}

async fn authenticate<Auth, Api>(
    auth: &Authenticator,
    auth_client: Auth,
    mut api_client: Api,
) -> anyhow::Result<()>
where
    Auth: AuthenticatorClientTrait,
    Api: CloudApi,
{
    let method = RequestMethod::Put;
    let params = api::enroll::RequestParams;
    let enroll_tokens = match auth {
        Authenticator::Auth0 => auth_client.auth0().await?,
    };
    match api_client
        .send::<_, _, ()>(method, params, enroll_tokens)
        .await
    {
        Ok(_) => {
            println!("Enrolled successfully");
            Ok(())
        }
        Err(err) => {
            tracing::error!("{:?}", err);
            Err(anyhow!("Failed to complete enrollment process"))
        }
    }
}

#[derive(Clone, Debug, Args)]
pub struct EnrollCommandArgs {
    /// Ockam's cloud node address
    #[clap(display_order = 1000)]
    pub cloud_addr: String,
    #[clap(display_order = 1001, arg_enum, default_value = "auth0")]
    pub authenticator: Authenticator,
    #[clap(display_order = 1002, long, default_value = "default")]
    pub vault: String,
    #[clap(display_order = 1003, long, default_value = "default")]
    pub identity: String,
    #[clap(display_order = 1004, long)]
    pub overwrite: bool,
}

impl<'a> From<&'a EnrollCommandArgs> for IdentityOpts {
    fn from(other: &'a EnrollCommandArgs) -> Self {
        Self {
            overwrite: other.overwrite,
        }
    }
}

#[derive(clap::ArgEnum, Clone, Debug)]
pub enum Authenticator {
    Auth0,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub(crate) trait AuthenticatorClientTrait {
    async fn auth0(&self) -> anyhow::Result<api::enroll::Auth0Tokens>;
}

pub(crate) struct AuthenticatorClient;

mod auth0 {
    use reqwest::StatusCode;
    use tokio_retry::{strategy::ExponentialBackoff, Retry};

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

    #[async_trait::async_trait]
    impl AuthenticatorClientTrait for AuthenticatorClient {
        async fn auth0(&self) -> anyhow::Result<api::enroll::Auth0Tokens> {
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
                .await?;
                match res.status() {
                    StatusCode::OK => {
                        let res = res.json::<DeviceCodeResponse>().await.map_err(|err| {
                            anyhow!("failed to deserialize device code response [err={err}]")
                        })?;
                        tracing::info!("device code received: {res:#?}");
                        res
                    }
                    _ => {
                        let err = anyhow!(
                            "couldn't get device code [response={:#?}]",
                            res.text().await?
                        );
                        return Err(err);
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
                tracing::warn!(
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
                    .await?;
                match res.status() {
                    StatusCode::OK => {
                        tokens_res =
                            res.json::<api::enroll::Auth0Tokens>()
                                .await
                                .map_err(|err| {
                                    anyhow!("failed to deserialize tokens response [err={err}]")
                                })?;
                        tracing::info!("tokens received [tokes={tokens_res:#?}]");
                        return Ok(tokens_res);
                    }
                    _ => {
                        let err_res = res.json::<TokensErrorResponse>().await?;
                        match err_res.error.as_str() {
                            "authorization_pending" | "invalid_request" | "slow_down" => {
                                tracing::info!("tokens not yet received [err={err_res:#?}]",);
                                tokio::time::sleep(tokio::time::Duration::from_secs(
                                    device_code_res.interval as u64,
                                ))
                                .await;
                                continue;
                            }
                            _ => {
                                let err_msg =
                                    format!("failed to receive tokens [err={err_res:#?}]",);
                                tracing::debug!("{}", err_msg);
                                return Err(anyhow!(err_msg));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use fake::{Fake, Faker};
    use mockall::predicate::*;

    use crate::api::MockCloudApi;

    use super::*;

    mod node_api {
        use crate::api::tests::node_api::*;

        use super::*;

        #[ockam::test(crate = "ockam")]
        async fn auth0__can_send_payload(ctx: &mut ockam::Context) -> ockam::Result<()> {
            let (mut node_api, worker_name) = setup_node_api(ctx).await?;
            let payload: api::enroll::Auth0Tokens = Faker.fake();
            send_payload::<api::enroll::Auth0Tokens>(&mut node_api, &worker_name, payload).await?;
            ctx.stop().await?;
            Ok(())
        }
    }

    mod auth0 {
        use super::*;

        // TODO: add tests for the auth0 internals using mockito
        // async fn internals__happy_path() {}
        // async fn internals__err_if_device_token_request_fails() {}
        // async fn internals__err_if_tokens_request_fails() {}

        #[tokio::test]
        async fn happy_path() -> anyhow::Result<()> {
            let req_params = api::enroll::RequestParams;
            let expected_creds: api::enroll::Auth0Tokens = Faker.fake();
            let mut auth_client = MockAuthenticatorClientTrait::new();
            let moved_expected_creds = expected_creds.clone();
            auth_client
                .expect_auth0()
                .times(1)
                .return_once(move || Ok(moved_expected_creds));

            let mut api_client = MockCloudApi::new();
            api_client
                .expect_send::<_, _, ()>()
                .with(eq(RequestMethod::Put), eq(req_params), eq(expected_creds))
                .times(1)
                .returning(|_, _, _| Ok(Some(())));

            authenticate(&Authenticator::Auth0, auth_client, api_client).await?;

            Ok(())
        }

        #[tokio::test]
        async fn err_if_auth0_flow_fails() -> anyhow::Result<()> {
            let mut auth_client = MockAuthenticatorClientTrait::new();
            auth_client
                .expect_auth0()
                .times(1)
                .return_once(move || Err(anyhow!("error")));

            let mut api_client = MockCloudApi::new();
            api_client
                .expect_send::<api::enroll::RequestParams, api::enroll::Auth0Tokens, ()>()
                .never();

            authenticate(&Authenticator::Auth0, auth_client, api_client)
                .await
                .expect_err("should fail");

            Ok(())
        }

        #[tokio::test]
        async fn err_if_authentication_fails() -> anyhow::Result<()> {
            let req_params = api::enroll::RequestParams;
            let expected_creds: api::enroll::Auth0Tokens = Faker.fake();
            let mut auth_client = MockAuthenticatorClientTrait::new();
            let moved_expected_creds = expected_creds.clone();
            auth_client
                .expect_auth0()
                .times(1)
                .return_once(move || Ok(moved_expected_creds));

            let mut api_client = MockCloudApi::new();
            api_client
                .expect_send::<_, _, ()>()
                .with(eq(RequestMethod::Put), eq(req_params), eq(expected_creds))
                .times(1)
                .returning(|_, _, _| {
                    Err(ockam::Error::new_unknown(
                        ockam::errcode::Origin::Application,
                        anyhow!("error"),
                    ))
                });

            authenticate(&Authenticator::Auth0, auth_client, api_client)
                .await
                .expect_err("should fail");

            Ok(())
        }
    }
}
