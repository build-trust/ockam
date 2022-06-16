use ockam::abac::{Action, Resource, Subject};
use ockam::{Address, Context, Message, Result, Routed, Worker};
use serde::{Deserialize, Serialize};

use crate::{
    AbacPolicyRequest, AbacPolicyResponse, AuthenticatedTableRequest, AuthenticatedTableResponse,
};

/// Abac Authorization Request Message type
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct AbacAuthorizationRequest(pub Subject, pub Resource, pub Action);

/// Abac Authorization Response Message type
#[derive(Debug, Serialize, Deserialize, Message)]
pub struct AbacAuthorizationResponse(pub bool);

/// A simple ABAC Authorization worker
pub struct AbacAuthorizationWorker {
    authenticated_table_address: Address,
    policy_address: Address,
}

impl AbacAuthorizationWorker {
    pub fn new<A: Into<Address>>(authenticated_table_address: A, policy_address: A) -> Self {
        Self {
            authenticated_table_address: authenticated_table_address.into(),
            policy_address: policy_address.into(),
        }
    }
}

#[ockam::worker]
impl Worker for AbacAuthorizationWorker {
    type Context = Context;
    type Message = AbacAuthorizationRequest;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();
        let AbacAuthorizationRequest(subject, resource, action) = msg.body();

        println!(
            "\nAbacAuthorizationWorker performing authorization for: {:?}, {:?}, {:?}",
            subject, resource, action
        );

        // Optional: enrich subject attributes from some authenticated table
        let AuthenticatedTableResponse(subject) = ctx
            .send_and_receive(
                self.authenticated_table_address.clone(),
                AuthenticatedTableRequest(subject),
            )
            .await?;

        println!("\nAbacAuthorizationWorker enriched subject: {:?}", subject);

        // retrieve policy for resource, action
        let response = ctx
            .send_and_receive(
                self.policy_address.clone(),
                AbacPolicyRequest(resource.clone(), action.clone()),
            )
            .await?;

        // perform authorization request for subject, resource, action
        let decision = match response {
            AbacPolicyResponse(Some(policy)) => {
                println!("\nAbacAuthorizationWorker applying policy: {:?}\n", policy);
                policy.evaluate(&subject, &resource, &action)
            }
            _ => {
                println!("\nAbacAuthorizationWorker failed to resolve policy");
                false
            }
        };

        ctx.send(return_route, AbacAuthorizationResponse(decision))
            .await
    }
}
