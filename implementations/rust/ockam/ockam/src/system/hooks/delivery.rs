use crate::{Context, OckamError, OckamMessage, Result, Routed, SystemHandler};
use ockam_core::{
    async_trait,
    compat::{boxed::Box, collections::BTreeMap, string::String},
    Address, Decodable, Encodable,
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
    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<OckamMessage>) -> Result<()> {
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
                    .generic_data("ockam.pipe.ack_id", ack_id.encode()?);
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
                    .generic
                    .as_ref()
                    .and_then(|data| data.get("ockam.pipe.ack_id"))
                    .and_then(|id| Address::decode(id).ok())
                    .unwrap();
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
pub struct ReceiverConfirm {}

#[async_trait]
impl SystemHandler<Context, OckamMessage> for ReceiverConfirm {
    async fn initialize(
        &mut self,
        ctx: &mut Context,
        routes: &mut BTreeMap<String, Address>,
    ) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<OckamMessage>) -> Result<()> {
        Ok(())
    }
}
