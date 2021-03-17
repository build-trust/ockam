use crate::channel::ChannelMessage;
use crate::ChannelError;
use crate::KeyExchangeMessage;
use async_trait::async_trait;
use ockam::{Address, Context, Worker};
use ockam_core::Result;
use ockam_key_exchange_core::KeyExchanger;
use ockam_key_exchange_xx::Initiator;

pub struct XInitiator {
    initiator: Option<Initiator>,
    channel_address: Address,
}

impl XInitiator {
    pub fn new(initiator: Initiator, channel_address: Address) -> Self {
        XInitiator {
            initiator: Some(initiator),
            channel_address,
        }
    }
}

#[async_trait]
impl Worker for XInitiator {
    type Message = KeyExchangeMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let initiator;
        if let Some(i) = self.initiator.as_mut() {
            initiator = i;
        } else {
            return Err(ChannelError::InvalidInternalState.into());
        }

        let m1_payload = initiator.process(&[])?;
        let m1 = ChannelMessage::InitiationMessage(m1_payload);

        ctx.send_message(self.channel_address.clone(), m1).await?;
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            KeyExchangeMessage::ChannelMessage(payload) => {
                let initiator;
                if let Some(i) = self.initiator.as_mut() {
                    initiator = i;
                } else {
                    return Err(ChannelError::InvalidInternalState.into());
                }

                // discard any payload and get the next message
                let _ = initiator.process(&payload)?;

                if !initiator.is_complete() {
                    let m_payload = initiator.process(&[])?;

                    let reply = ChannelMessage::KeyExchangeMessage(m_payload);

                    ctx.send_message(self.channel_address.clone(), reply)
                        .await?;
                }

                if initiator.is_complete() {
                    let initiator;
                    if let Some(i) = self.initiator.take() {
                        initiator = i;
                    } else {
                        return Err(ChannelError::InvalidInternalState.into());
                    }

                    let completed_key_exchange = initiator.finalize()?;

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
