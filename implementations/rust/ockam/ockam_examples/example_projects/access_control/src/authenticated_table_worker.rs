use ockam::abac::{AbacAttributeStorage, Subject};
use ockam::{Context, Message, Result, Routed, Worker};
use serde::{Deserialize, Serialize};

/// A simple authenticated table service which serves subject
/// attributes to a requester
pub struct AuthenticatedTableWorker<B> {
    backend: B,
}

impl<B> AuthenticatedTableWorker<B>
where
    B: AbacAttributeStorage,
{
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
}

#[derive(Debug, Serialize, Deserialize, Message)]
pub struct AuthenticatedTableRequest(pub Subject);

#[derive(Debug, Serialize, Deserialize, Message)]
pub struct AuthenticatedTableResponse(pub Subject);

#[ockam::worker]
impl<B> Worker for AuthenticatedTableWorker<B>
where
    B: AbacAttributeStorage,
{
    type Context = Context;
    type Message = AuthenticatedTableRequest;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let return_route = msg.return_route();

        // get the subject
        let AuthenticatedTableRequest(mut subject) = msg.body();

        // get all attributes for the subject
        let attributes = self.backend.get_subject_attributes(&subject).await?;

        // enrich subject attributes
        subject.extend(attributes);

        // return the subject to the requester
        ctx.send(return_route, AuthenticatedTableResponse(subject))
            .await
    }
}
