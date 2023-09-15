use ockam::identity::OneTimeCode;
use ockam_core::api::Request;
use ockam_core::Result;
use ockam_node::RpcClient;

pub struct TokenAcceptorClient(RpcClient);

impl TokenAcceptorClient {
    pub fn new(client: RpcClient) -> Self {
        TokenAcceptorClient(client)
    }

    pub async fn present_token(&self, c: &OneTimeCode) -> Result<()> {
        self.0
            .request_no_resp_body(&Request::post("/").body(c))
            .await
    }
}
