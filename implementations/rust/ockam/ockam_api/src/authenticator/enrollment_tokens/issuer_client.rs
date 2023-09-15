use core::str;
use ockam::identity::OneTimeCode;
use ockam_core::api::Request;
use ockam_core::Result;
use ockam_node::RpcClient;
use std::collections::HashMap;
use std::time::Duration;

use crate::authenticator::direct::types::CreateToken;

pub struct TokenIssuerClient(RpcClient);

impl TokenIssuerClient {
    pub fn new(client: RpcClient) -> Self {
        TokenIssuerClient(client)
    }

    pub async fn create_token(
        &self,
        attributes: HashMap<&str, &str>,
        duration: Option<Duration>,
    ) -> Result<OneTimeCode> {
        self.0
            .request(
                &Request::post("/").body(
                    CreateToken::new()
                        .with_attributes(attributes)
                        .with_duration(duration),
                ),
            )
            .await
    }
}
