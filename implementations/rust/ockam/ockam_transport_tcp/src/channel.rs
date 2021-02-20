use crate::{ChannelError, Connection, TransportError};
use core::sync::atomic::AtomicI32;
use ockam_core::lib::net::TcpListener;
use ockam_key_exchange_core::{CompletedKeyExchange, KeyExchanger};
use ockam_key_exchange_xx::XXNewKeyExchanger;
use ockam_router::message::{RouterMessage, ROUTER_MSG_M2, ROUTER_MSG_REQUEST_CHANNEL};
use rand::prelude::*;
use std::sync::{Arc, Mutex};

pub enum ExchangerRole {
    Initiator,
    Responder,
}

#[derive(Debug)]
pub struct Channel {
    encrypt_addr: Vec<u8>,
    decrypt_addr: Vec<u8>,
    key: Option<CompletedKeyExchange>,
}

impl Channel {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let random = rng.gen::<u32>();
        let encrypt_addr = random.to_le_bytes().to_vec();
        let random = rng.gen::<u32>();
        let decrypt_addr = random.to_le_bytes().to_vec();
        let key = None;
        Self {
            encrypt_addr,
            decrypt_addr,
            key,
        }
    }
    pub async fn initialize(
        &mut self,
        mut exchanger: Box<dyn KeyExchanger>,
        mut connection: Box<dyn Connection>,
        role: ExchangerRole,
    ) -> ockam_core::Result<()> {
        let mut m_expected = 0;

        match role {
            ExchangerRole::Initiator => {
                let mut m = RouterMessage::new();
                m.payload.insert(0, ROUTER_MSG_REQUEST_CHANNEL);
                m.payload = exchanger.process(&[]).unwrap();
                m_expected = ROUTER_MSG_M2;
            }
            ExchangerRole::Responder => {
                m_expected = ROUTER_MSG_REQUEST_CHANNEL;
            }
        }

        while !exchanger.is_complete() {
            // 1. wait for a message to arrive.
            let m = connection.receive_message().await?;
            // 2. verify that it's the expected message
            if m.payload[0] != m_expected {
                return Err(ChannelError::KeyExchange);
            }

            // 3. discard whatever payload there was
            let _ = exchanger.process(&m.payload)?;

            // 4. get and send the next message
            let mut m = RouterMessage::new();
            m.payload = exchanger.process(&[])?;
            m_expected += 1;
            m.payload.insert(0, m_expected);
            connection.send_message(m).await?;

            m_expected += 1;
        }

        self.key = Some(exchanger.finalize()?);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::listener::TcpListener;
    use crate::Channel;
    use ockam_core::lib::net::SocketAddr;
    use ockam_core::lib::str::FromStr;
    use ockam_key_exchange_xx::XXNewKeyExchanger;
    use ockam_vault::SoftwareVault;
    use std::sync::{Arc, Mutex};
    use tokio::runtime::Builder;

    async fn initiator_key_exchange() {}

    async fn responder_key_exchange() {
        let mut listener = TcpListener::create(SocketAddr::from_str("127.0.0.1:4051").unwrap())
            .await
            .unwrap();
        let connection = listener.accept().await.unwrap();

        let vault_initiator = Arc::new(Mutex::new(SoftwareVault::default()));
        let vault_responder = Arc::new(Mutex::new(SoftwareVault::default()));
        let key_exchanger =
            XXNewKeyExchanger::new(vault_initiator.clone(), vault_responder.clone());
    }

    #[test]
    fn test_exchange() {
        let runtime = Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();

        runtime.block_on(async {
            let j1 = responder_key_exchange();
        });
    }
}
