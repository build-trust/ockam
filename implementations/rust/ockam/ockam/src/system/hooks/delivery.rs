use crate::{Context, OckamError, OckamMessage, Result, Routed, SystemHandler};
use ockam_core::{
    async_trait,
    compat::{boxed::Box, collections::BTreeMap, string::String, vec::Vec},
    Address, Any, Decodable, Encodable,
};

#[derive(Clone, Default)]
pub struct SenderConfirm {
    next: Option<Address>,
    journal: BTreeMap<Address, OckamMessage>,
}

fn to_str(v: &[u8]) -> &str {
    core::str::from_utf8(v).unwrap()
}

#[async_trait]
impl SystemHandler<Context, OckamMessage> for SenderConfirm {
    async fn initialize(
        &mut self,
        ctx: &mut Context,
        routes: &mut BTreeMap<String, Address>,
    ) -> Result<()> {
        self.next = Some(
            routes
                .remove("default")
                .ok_or(OckamError::SystemInvalidConfiguration)?,
        );
        Ok(())
    }

    // The sender confirm hook stores a reference for the sent-message
    // and registers a delayed event to re-check whether we received a
    // confirmation or not.
    async fn handle_message(
        &mut self,
        self_addr: Address,
        ctx: &mut Context,
        msg: Routed<OckamMessage>,
    ) -> Result<()> {
        match msg
            .generic
            .as_ref()
            .and_then(|data| data.get("ockam.pipe.type"))
            .map(|tt| to_str(tt))
        {
            // For a message with no type we register a delayed event
            // and forward it to the next step in the system
            None => {
                let ack_id = Address::random(0);
                let inner_msg = msg.body();
                let outer_msg = OckamMessage::wrap(inner_msg)?
                    .scope_data(ack_id.encode()?)
                    .scope_data(self_addr.encode()?);
                self.journal.insert(ack_id, outer_msg.clone());

                // TODO: register notify event

                // Forward the new message to the next address
                ctx.send(self.next.as_ref().unwrap().clone(), outer_msg)
                    .await?;
            }

            // For any "ACK" we receive we can delete the
            // corresponding ACK id from the journal
            Some(ref tt) if tt == &"ockam.pipe.ack" => {
                let ack_id = msg
                    .scope
                    .get(0)
                    .and_then(|id| Address::decode(id).ok())
                    .unwrap();
                info!("Received ACK for message: {}", ack_id);
                self.journal.remove(&ack_id);
            }

            // When receiving a notify message we check whether an ACK
            // handle still exists, and if it does we re-send the
            // message
            Some(ref tt) if tt == &"ockam.pipe.resend_notify" => {
                let ack_id = msg
                    .generic
                    .as_ref()
                    .and_then(|data| data.get("ockam.pipe.ack_id"))
                    .and_then(|id| Address::decode(id).ok())
                    .unwrap();

                if let Some(msg) = self.journal.remove(&ack_id) {
                    // TODO: register new notify event
                    ctx.send(self.next.as_ref().unwrap().clone(), msg).await?;
                }
            }

            // Any other type is an invalid message that will be dropped
            Some(tt) => {
                error!("Invalid OckamMessage type '{}'.  dropping message", tt);
            }
        }

        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct ReceiverConfirm {
    next: Option<Address>,
}

fn addr_from_scope(idx: usize, scope: &[Vec<u8>]) -> Result<Address> {
    let vec = scope.get(idx).ok_or(OckamError::InvalidParameter)?;
    Address::decode(vec)
}

#[async_trait]
impl SystemHandler<Context, OckamMessage> for ReceiverConfirm {
    async fn initialize(
        &mut self,
        ctx: &mut Context,
        routes: &mut BTreeMap<String, Address>,
    ) -> Result<()> {
        self.next = Some(
            routes
                .remove("default")
                .ok_or(OckamError::SystemInvalidConfiguration)?,
        );
        Ok(())
    }

    async fn handle_message(
        &mut self,
        self_addr: Address,
        ctx: &mut Context,
        msg: Routed<OckamMessage>,
    ) -> Result<()> {
        // First grab the return route so we may edit it later
        let mut return_route = msg.return_route();

        // Grab the scope metadata from the message
        let inner = msg.body();
        let ack_id = addr_from_scope(0, &inner.scope)?;
        let ack_addr = addr_from_scope(1, &inner.scope)?;

        // Send an ACK back to the sender
        let ack = OckamMessage::new(Any)?
            .scope_data(ack_id.encode()?)
            .generic_data("ockam.pipe.type", "ockam.pipe.ack".as_bytes().to_vec());
        ctx.send(return_route.modify().pop_back().append(ack_addr), ack)
            .await?;

        // Then peel the message and forward to the next hop
        ctx.send(self.next.as_ref().unwrap().clone(), inner.peel()?)
            .await?;
        Ok(())
    }
}
