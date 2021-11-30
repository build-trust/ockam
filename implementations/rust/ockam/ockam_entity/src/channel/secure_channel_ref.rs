use ockam_core::{route, Address};
use ockam_core::{Message, Result};
use ockam_node::Context;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Message)]
pub(crate) enum SecureChannelApiRequest {
    GetAuthHash,
}

#[derive(Serialize, Deserialize, Message)]
pub(crate) enum SecureChannelApiResponse {
    GetAuthHash([u8; 32]),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SecureChannelRef {
    api_address: Address,
    local_address: Address,
}

impl SecureChannelRef {
    pub fn new(api_address: Address, local_address: Address) -> Self {
        Self {
            api_address,
            local_address,
        }
    }
}

impl SecureChannelRef {
    pub async fn get_auth_hash(&self, ctx: &Context) -> Result<[u8; 32]> {
        let mut child_ctx = ctx.new_context(Address::random(0)).await?;
        child_ctx
            .send(
                route![self.api_address.clone()],
                SecureChannelApiRequest::GetAuthHash,
            )
            .await?;

        let SecureChannelApiResponse::GetAuthHash(hash) = child_ctx
            .receive::<SecureChannelApiResponse>()
            .await?
            .take()
            .body();

        Ok(hash)
    }
}

impl From<SecureChannelRef> for Address {
    fn from(r: SecureChannelRef) -> Self {
        r.local_address
    }
}
