use crate::{SecureChannelTrustInfo, TrustPolicy};
use ockam_core::compat::boxed::Box;
use ockam_core::{
    async_trait::async_trait, Address, AsyncTryClone, Handle, Message, NodeContext, Result, Routed,
    Worker,
};
use ockam_message_derive::Message;
use serde::{Deserialize, Serialize};

pub struct TrustPolicyImpl<C> {
    handle: Handle<C>,
}

#[async_trait]
impl<C: NodeContext> AsyncTryClone for TrustPolicyImpl<C> {
    async fn async_try_clone(&self) -> Result<Self> {
        Ok(Self {
            handle: self.handle.async_try_clone().await?,
        })
    }
}

impl<C: NodeContext> TrustPolicyImpl<C> {
    pub fn new(handle: Handle<C>) -> Self {
        TrustPolicyImpl { handle }
    }
    pub async fn create_using_worker(ctx: &C, address: &Address) -> Result<Self> {
        let handle = Handle::new(ctx.new_context(Address::random(0)).await?, address.clone());

        Ok(Self::new(handle))
    }

    pub async fn create_using_impl(ctx: &C, trust_policy: impl TrustPolicy) -> Result<Self> {
        let address = Self::create_worker(ctx, trust_policy).await?;
        Self::create_using_worker(ctx, &address).await
    }

    pub async fn create_worker(ctx: &C, trust_policy: impl TrustPolicy) -> Result<Address> {
        let address = Address::random(0);

        ctx.start_worker(address.clone().into(), TrustPolicyWorker::new(trust_policy))
            .await?;

        Ok(address)
    }
}

#[async_trait]
impl<C: NodeContext> TrustPolicy for TrustPolicyImpl<C> {
    async fn check(&self, trust_info: &SecureChannelTrustInfo) -> Result<bool> {
        let response: TrustPolicyResponse = self
            .handle
            .call(TrustPolicyRequest {
                info: trust_info.clone(),
            })
            .await?;

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

#[derive(Serialize, Deserialize, Message)]
pub struct TrustPolicyRequest {
    pub info: SecureChannelTrustInfo,
}

#[derive(Serialize, Deserialize, Message)]
pub struct TrustPolicyResponse {
    pub res: bool,
}

#[async_trait]
impl<T: TrustPolicy, C: NodeContext> Worker<C> for TrustPolicyWorker<T> {
    type Message = TrustPolicyRequest;

    async fn handle_message(&mut self, ctx: &mut C, msg: Routed<Self::Message>) -> Result<()> {
        let route = msg.return_route();
        let msg = msg.body();

        let res = self.trust_policy.check(&msg.info).await?;
        ctx.send(route, TrustPolicyResponse { res }).await?;

        Ok(())
    }
}
