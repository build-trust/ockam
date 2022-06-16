use ockam::abac::{AbacPolicyStorage, Action, Conditional, Resource};
use ockam::{Context, Message, Result, Routed, Worker};
use serde::{Deserialize, Serialize};

/// A simple Policy worker which serves ABAC policies to a requester
pub struct AbacPolicyWorker<B> {
    backend: B,
}

impl<B> AbacPolicyWorker<B>
where
    B: AbacPolicyStorage,
{
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
}

#[derive(Debug, Serialize, Deserialize, Message)]
pub struct AbacPolicyRequest(pub Resource, pub Action);

#[derive(Debug, Serialize, Deserialize, Message)]
pub struct AbacPolicyResponse(pub Option<Conditional>);

#[ockam::worker]
impl<B> Worker for AbacPolicyWorker<B>
where
    B: AbacPolicyStorage,
{
    type Context = Context;
    type Message = AbacPolicyRequest;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();

        // get the resource and action
        let AbacPolicyRequest(resource, action) = msg.body();

        // get the policy matching the resource and action
        let policy = self.backend.get_policy(&resource, &action).await?;

        // return it to the requester
        ctx.send(return_route, AbacPolicyResponse(policy)).await
    }
}
