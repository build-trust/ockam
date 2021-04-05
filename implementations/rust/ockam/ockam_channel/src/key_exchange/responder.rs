use crate::key_exchange::{KeyExchangeRequestMessage, KeyExchangeResponseMessage, Keys};
use crate::SecureChannelError;
use async_trait::async_trait;
use ockam::{Context, Result, Worker};
use ockam_core::Routed;
use ockam_key_exchange_core::KeyExchanger;
use ockam_key_exchange_xx::Responder;

// TODO: Move to key exchange crate
pub(crate) struct XResponder {
    responder: Option<Responder>,
}

impl XResponder {
    pub(crate) fn new(responder: Responder) -> Self {
        XResponder {
            responder: Some(responder),
        }
    }
}

#[async_trait]
impl Worker for XResponder {
    type Message = KeyExchangeRequestMessage;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        // TODO: copy&paste from initiator
        let reply = msg.reply();
        match msg.take() {
            KeyExchangeRequestMessage::InitiatorFirstMessage { req_id: _ } => {
                return Err(SecureChannelError::InvalidInternalState.into());
            }
            KeyExchangeRequestMessage::Payload { req_id, payload } => {
                let responder;
                if let Some(i) = self.responder.as_mut() {
                    responder = i;
                } else {
                    return Err(SecureChannelError::InvalidInternalState.into());
                }

                // discard any payload and get the next message
                let _ = responder.process(&payload)?;

                let new_payload = if !responder.is_complete() {
                    Some(responder.process(&[])?)
                } else {
                    None
                };

                let mut should_stop = false;

                let keys = if responder.is_complete() {
                    let responder;
                    if let Some(r) = self.responder.take() {
                        responder = r;
                    } else {
                        return Err(SecureChannelError::InvalidInternalState.into());
                    }

                    let keys = responder.finalize()?;
                    let keys = Keys::new(
                        keys.h().clone(),
                        keys.encrypt_key().index(),
                        keys.decrypt_key().index(),
                    );
                    should_stop = true;

                    Some(keys)
                } else {
                    None
                };

                ctx.send_message(
                    reply,
                    KeyExchangeResponseMessage::new(req_id, new_payload, keys),
                )
                .await?;

                if should_stop {
                    ctx.stop_worker(ctx.address()).await?;
                }
            }
        }

        Ok(())
    }
}
