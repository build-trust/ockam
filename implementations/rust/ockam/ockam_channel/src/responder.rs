use crate::{ChannelError, Connection};
use ockam::{Address, Context, Result, Worker};
use ockam_key_exchange_core::{CompletedKeyExchange, KeyExchanger, NewKeyExchanger};
use ockam_key_exchange_xx::{Initiator, Responder, XXNewKeyExchanger};
use ockam_router::message::{
    RouteableAddress, RouterMessage, ROUTER_MSG_M2, ROUTER_MSG_REQUEST_CHANNEL,
};
use ockam_router::{Route, RouteTransportMessage, RouterAddress, TransportMessage, ROUTER_ADDRESS};
use rand::prelude::*;
use std::sync::Arc;

pub struct XResponder {
    m_expected: u8,
    connection_address: Address,
    parent: Address,
    responder: Responder,
    route: Route,
}

impl Worker for XResponder {
    type Message = RouteTransportMessage;
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        let mut responder = self.exchanger.responder();
        ctx.send_message(ROUTER_ADDRESS, RouteTransportMessage::Route(m1))
            .await?;
        Ok(())
    }

    fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Self::Context, msg: Self::Message) -> Result<()> {
        return match msg {
            RouteTransportMessage::Route(mut msg) => {
                return if !self.initiator.is_complete() {
                    let m_received = msg.payload.remove(0);
                    if m_received != self.m_expected {
                        Err(ChannelError::KeyExchange.into())
                    }
                    // discard any payload and get the next message
                    let _ = self.initiator.process(&msg.payload)?;
                    let m = self.initiator.process(&[])?;

                    let mut reply = TransportMessage::new();
                    reply.onward_route = msg.return_route.clone();
                    reply.return_address(RouteableAddress::Local(ctx.address().to_vec()));
                    m.insert(0, self.m_expected + 1);
                    self.m_expected += 2;
                    reply.payload = m;
                    ctx.send_message(ROUTER_ADDRESS, RouteTransportMessage::Route(reply))
                        .await?;
                    Ok(())
                } else {
                    Err(ChannelError::KeyExchange.into())
                };
            }
            _ => Ok(()),
        };
    }
}

#[derive(Debug)]
pub struct Channel {
    encrypt_addr: Vec<u8>,
    decrypt_addr: Vec<u8>,
    initiator_key: Option<CompletedKeyExchange>,
    responder_key: Option<CompletedKeyExchange>,
}

impl Default for Channel {
    fn default() -> Self {
        Self::new()
    }
}

impl Channel {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let random = rng.gen::<u32>();
        let encrypt_addr = random.to_le_bytes().to_vec();
        let random = rng.gen::<u32>();
        let decrypt_addr = random.to_le_bytes().to_vec();
        Self {
            encrypt_addr,
            decrypt_addr,
            initiator_key: None,
            responder_key: None,
        }
    }

    pub async fn initialize_responder(
        &mut self,
        exchanger: Box<XXNewKeyExchanger>,
        connection: Address,
        mut ctx: Arc<Mut<Context>>,
    ) -> Result<()> {
        let mut responder = exchanger.responder();
        let mut m_expected = ROUTER_MSG_REQUEST_CHANNEL;

        while !responder.is_complete() {
            // 1. wait for a message to arrive.
            let mut m1 = connection.receive_message().await?;
            // 2. verify that it's the expected message
            if m1.payload[0] != m_expected {
                return Err(ChannelError::KeyExchange.into());
            }
            m1.payload.remove(0);

            // 3. discard whatever payload there was
            let _ = responder.process(&m1.payload)?;
            if responder.is_complete() {
                break;
            }

            // 4. construct and send the next message
            let mut m2 = RouterMessage::new();
            m2.onward_route = m1.return_route.clone();
            m2.payload = responder.process(&[])?;
            m_expected += 1;
            m2.payload.insert(0, m_expected);
            connection.send_message(m2).await?;
            m_expected += 1;
        }

        let key = responder.finalize()?;
        self.initiator_key = Some(key);

        println!("Responder successful!!!");

        Ok(())
    }

    pub async fn initialize_initiator(
        &mut self,
        exchanger: Box<XXNewKeyExchanger>,
        mut connection: Box<dyn Connection>,
    ) -> Result<()> {
        let mut initiator = exchanger.initiator();
        let mut m_expected = ROUTER_MSG_M2;

        while !initiator.is_complete() {
            // 1. construct request-channel message
            let mut m1 = RouterMessage::new();
            m1.onward_address(RouteableAddress::Local(vec![]));
            m1.return_address(RouteableAddress::Local(vec![]));
            m1.payload = initiator.process(&[])?;
            m1.payload.insert(0, ROUTER_MSG_REQUEST_CHANNEL);
            let m1_return = m1.return_route.clone();
            connection.send_message(m1).await?;

            let mut m2 = connection.receive_message().await?;
            // 2. verify that it's the expected message
            if m2.payload[0] != m_expected {
                return Err(ChannelError::KeyExchange.into());
            }
            m2.payload.remove(0);

            // 3. discard whatever payload there was
            let _ = initiator.process(&m2.payload)?;
            if initiator.is_complete() {
                break;
            }

            // 4. construct and send the next message
            let mut m3 = RouterMessage::new();
            m3.onward_route = m1_return;
            m3.return_address(RouteableAddress::Local(vec![]));
            m3.payload = initiator.process(&[])?;
            m_expected += 1;
            m3.payload.insert(0, m_expected);
            connection.send_message(m3).await?;
            m_expected += 1;
        }

        let key = initiator.finalize()?;
        self.initiator_key = Some(key);

        println!("Initiator successful!!");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::listener::TcpListener;
    use crate::{Channel, TcpConnection};
    use ockam_core::lib::net::SocketAddr;
    use ockam_core::lib::str::FromStr;
    use ockam_key_exchange_xx::XXNewKeyExchanger;
    use ockam_vault::SoftwareVault;
    use std::sync::{Arc, Mutex};
    use tokio::runtime::Builder;

    async fn initiator_key_exchange() {
        let mut connection = TcpConnection::create(SocketAddr::from_str("127.0.0.1:4060").unwrap());
        connection.connect().await.unwrap();

        let vault_initiator = Arc::new(Mutex::new(SoftwareVault::default()));
        let vault_responder = Arc::new(Mutex::new(SoftwareVault::default()));
        let key_exchanger =
            XXNewKeyExchanger::new(vault_initiator.clone(), vault_responder.clone());

        let mut channel = Channel::new();

        channel
            .initialize_initiator(Box::new(key_exchanger), connection)
            .await
            .unwrap();
    }

    async fn responder_key_exchange() {
        let mut listener = TcpListener::create(SocketAddr::from_str("127.0.0.1:4060").unwrap())
            .await
            .unwrap();
        let connection = listener.accept().await.unwrap();

        let vault_initiator = Arc::new(Mutex::new(SoftwareVault::default()));
        let vault_responder = Arc::new(Mutex::new(SoftwareVault::default()));
        let key_exchanger =
            XXNewKeyExchanger::new(vault_initiator.clone(), vault_responder.clone());

        let mut channel = Channel::new();
        channel
            .initialize_responder(Box::new(key_exchanger), connection)
            .await
            .unwrap();
    }

    #[test]
    fn test_exchange() {
        let runtime = Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();

        runtime.block_on(async {
            let j1 = tokio::task::spawn(async {
                let f = responder_key_exchange();
                f.await;
            });
            let j2 = tokio::task::spawn(async {
                let f = initiator_key_exchange();
                f.await;
            });
            let (r1, r2) = tokio::join!(j1, j2);
            if r1.is_err() {
                panic!("{:?}", r1);
            }
            if r2.is_err() {
                panic!("{:?}", r2);
            }
        });
    }
}
