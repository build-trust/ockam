use crate::{Handle, SecureChannelTrustInfo, TrustPolicy};
use async_trait::async_trait;
use ockam_core::{Address, Result, Routed, Worker};
use ockam_node::Context;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct TrustPolicyImpl {
    handle: Handle,
}

impl TrustPolicyImpl {
    pub fn new(handle: Handle) -> Self {
        TrustPolicyImpl { handle }
    }
}

impl TrustPolicyImpl {
    pub async fn create(ctx: &Context, trust_policy: impl TrustPolicy) -> Result<Address> {
        let address = Address::random(0);

        ctx.start_worker(address.clone(), TrustPolicyWorker::new(trust_policy))
            .await?;

        Ok(address)
    }
}

impl TrustPolicy for TrustPolicyImpl {
    fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        let response: TrustPolicyResponse = self.handle.call(TrustPolicyRequest {
            info: trust_info.clone(),
        })?;

        Ok(response.res)
    }
}

pub struct TrustPolicyWorker<T: TrustPolicy> {
    trust_policy: T,
}

impl<T: TrustPolicy> TrustPolicyWorker<T> {
    pub fn new(trust_policy: T) -> Self {
        TrustPolicyWorker { trust_policy }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TrustPolicyRequest {
    pub info: SecureChannelTrustInfo,
}

#[derive(Serialize, Deserialize)]
pub struct TrustPolicyResponse {
    pub res: bool,
}

#[async_trait]
impl<T: TrustPolicy> Worker for TrustPolicyWorker<T> {
    type Message = TrustPolicyRequest;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let route = msg.return_route();
        let msg = msg.body();

        let res = self.trust_policy.check(&msg.info)?;
        ctx.send(route, TrustPolicyResponse { res }).await?;

        Ok(())
    }
}
