use crate::channel::ChannelMessage;
use crate::channel_factory::{ChannelFactoryMessage, XX_CHANNEL_FACTORY_ADDRESS};
use async_trait::async_trait;
use ockam::{Address, Context, Worker};
use ockam_core::Result;
use ockam_router::RouteTransportMessage;
use serde::{Deserialize, Serialize};

pub const CHANNELS_FACADE_ADDRESS: &str = "channels_facade";

pub struct ChannelsFacade {}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ChannelsFacadeMessage {
    RequestNewChannel {
        channel_id: String,
        payload: Vec<u8>,
    },
    Forward {
        channel_id: String,
        payload: Vec<u8>,
    },
}

impl ChannelsFacade {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Worker for ChannelsFacade {
    type Message = RouteTransportMessage;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            RouteTransportMessage::Route(msg) => {
                let facade_msg: ChannelsFacadeMessage =
                    serde_bare::from_slice(msg.payload.as_slice()).unwrap();

                match facade_msg {
                    ChannelsFacadeMessage::Forward {
                        channel_id,
                        payload,
                    } => {
                        let channel_encrypted_address: Address =
                            format!("channel_enc/{}", channel_id)
                                .as_bytes()
                                .to_vec()
                                .into();

                        ctx.send_message(
                            channel_encrypted_address,
                            ChannelMessage::Forward(payload),
                        )
                        .await
                    }
                    ChannelsFacadeMessage::RequestNewChannel {
                        channel_id,
                        payload,
                    } => {
                        let create_channel_msg = ChannelFactoryMessage::create_responder_channel(
                            channel_id.clone(),
                            msg.return_route.addrs.get(0).unwrap().clone(),
                            payload,
                        );
                        ctx.send_message(XX_CHANNEL_FACTORY_ADDRESS, create_channel_msg)
                            .await
                    }
                }
            }
            RouteTransportMessage::Ping => unimplemented!(),
        };
    }
}
