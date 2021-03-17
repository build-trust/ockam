use crate::channel::{Channel, ChannelMessage};
use async_trait::async_trait;
use ockam::{Address, Context, Worker};
use ockam_core::Result;
use ockam_router::RouterAddress;
use ockam_transport_tcp::TcpWorkerMessage;
use serde::{Deserialize, Serialize};

pub const XX_CHANNEL_FACTORY_ADDRESS: &str = "xx_channel_factory";

pub struct XXChannelFactory {
    key_exchange_factory_address: Address,
}

impl XXChannelFactory {
    pub fn new(key_exchange_factory_address: Address) -> Self {
        XXChannelFactory {
            key_exchange_factory_address,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CreateInitiatorChannelMsg {
    channel_id: String,
    transport_address: RouterAddress,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct CreateResponderChannelMsg {
    channel_id: String,
    transport_address: RouterAddress,
    payload: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ChannelFactoryMessage {
    CreateInitiatorChannel(CreateInitiatorChannelMsg),
    CreateResponderChannel(CreateResponderChannelMsg),
    WaitForInitiator { transport_address: Address },
}

impl ChannelFactoryMessage {
    pub fn create_initiator_channel(
        channel_id: String,
        transport_address: RouterAddress,
    ) -> ChannelFactoryMessage {
        let msg = CreateInitiatorChannelMsg {
            channel_id,
            transport_address,
        };

        ChannelFactoryMessage::CreateInitiatorChannel(msg)
    }

    pub fn create_responder_channel(
        channel_id: String,
        transport_address: RouterAddress,
        payload: Vec<u8>,
    ) -> ChannelFactoryMessage {
        let msg = CreateResponderChannelMsg {
            channel_id,
            transport_address,
            payload,
        };

        ChannelFactoryMessage::CreateResponderChannel(msg)
    }

    pub fn wait_for_initiator(transport_address: Address) -> ChannelFactoryMessage {
        ChannelFactoryMessage::WaitForInitiator { transport_address }
    }
}

#[async_trait]
impl Worker for XXChannelFactory {
    type Message = ChannelFactoryMessage;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            ChannelFactoryMessage::CreateInitiatorChannel(msg) => {
                let channel_encrypted_address: Address = format!("channel_enc/{}", msg.channel_id)
                    .as_bytes()
                    .to_vec()
                    .into();
                let channel_key_exchange_address: Address =
                    format!("channel_kex/{}", msg.channel_id)
                        .as_bytes()
                        .to_vec()
                        .into();

                let channel = Channel::new(
                    self.key_exchange_factory_address.clone(),
                    msg.transport_address,
                    true,
                    msg.channel_id,
                    channel_key_exchange_address.clone(),
                );

                let channel_address_set: Vec<Address> =
                    vec![channel_encrypted_address, channel_key_exchange_address];

                ctx.start_worker(channel_address_set, channel).await
            }
            ChannelFactoryMessage::CreateResponderChannel(msg) => {
                let channel_encrypted_address: Address = format!("channel_enc/{}", msg.channel_id)
                    .as_bytes()
                    .to_vec()
                    .into();
                let channel_key_exchange_address: Address =
                    format!("channel_kex/{}", msg.channel_id)
                        .as_bytes()
                        .to_vec()
                        .into();

                let channel = Channel::new(
                    self.key_exchange_factory_address.clone(),
                    msg.transport_address,
                    false,
                    msg.channel_id,
                    channel_key_exchange_address.clone(),
                );

                let channel_address_set: Vec<Address> = vec![
                    channel_encrypted_address.clone(),
                    channel_key_exchange_address,
                ];

                ctx.start_worker(channel_address_set, channel).await?;

                ctx.send_message(
                    channel_encrypted_address,
                    ChannelMessage::Forward(msg.payload),
                )
                .await
            }
            ChannelFactoryMessage::WaitForInitiator { transport_address } => {
                ctx.send_message(transport_address, TcpWorkerMessage::Receive)
                    .await
            }
        };
    }
}
