use crate::channel::ChannelMessage;
use crate::{ChannelError, KeyExchangeMessage};
use async_trait::async_trait;
use ockam::{Address, Context, Result, Worker};
use ockam_key_exchange_core::KeyExchanger;
use ockam_key_exchange_xx::Responder;

pub struct XResponder {
    responder: Option<Responder>,
    channel_address: Address,
}

impl XResponder {
    pub fn new(responder: Responder, channel_address: Address) -> Self {
        XResponder {
            responder: Some(responder),
            channel_address,
        }
    }

    fn create_message(payload: Vec<u8>) -> ChannelMessage {
        ChannelMessage::KeyExchangeMessage(payload)
    }
}

#[async_trait]
impl Worker for XResponder {
    type Message = KeyExchangeMessage;
    type Context = Context;

    async fn initialize(&mut self, _ctx: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        // FIXME: copy&paste from initiator
        return match msg {
            KeyExchangeMessage::ChannelMessage(payload) => {
                let responder;
                if let Some(i) = self.responder.as_mut() {
                    responder = i;
                } else {
                    return Err(ChannelError::InvalidInternalState.into());
                }

                // discard any payload and get the next message
                let _ = responder.process(&payload)?;

                if !responder.is_complete() {
                    let m_payload = responder.process(&[])?;

                    let reply = Self::create_message(m_payload);

                    ctx.send_message(self.channel_address.clone(), reply)
                        .await?;
                }

                if responder.is_complete() {
                    let responder;
                    if let Some(i) = self.responder.take() {
                        responder = i;
                    } else {
                        return Err(ChannelError::InvalidInternalState.into());
                    }

                    let completed_key_exchange = responder.finalize()?;

                    ctx.send_message(
                        self.channel_address.clone(),
                        ChannelMessage::ExchangeComplete {
                            h: completed_key_exchange.h().clone(),
                            encrypt_key: completed_key_exchange.encrypt_key().index(),
                            decrypt_key: completed_key_exchange.decrypt_key().index(),
                        },
                    )
                    .await?;
                }

                Ok(())
            }
        };
    }
}
