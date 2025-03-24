use crate::portal::{PSQL_REQUEST_TLS_BIN, PSQL_RESPONSE_TLS_BIN};
use crate::TcpRegistry;
use log::{info, warn};
use ockam_core::{async_trait, compat::boxed::Box, Result};
use ockam_core::{Address, Processor};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use rustls::server::Acceptor;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tracing::{debug, error, instrument};

const BUFFER_LEN: usize = 63000;

pub(crate) struct TcpSniRootInletListenProcessor {
    registry: TcpRegistry,
    listener: TcpListener,
    buffer: Vec<u8>,
}

impl TcpSniRootInletListenProcessor {
    fn new(listener: TcpListener, registry: TcpRegistry) -> Self {
        Self {
            listener,
            registry,
            buffer: vec![0u8; BUFFER_LEN],
        }
    }

    #[instrument(skip_all, name = "TcpSniRootInletListenProcessor::start")]
    pub(crate) async fn start(
        ctx: &Context,
        registry: TcpRegistry,
        addr: SocketAddr,
    ) -> Result<Address> {
        let processor_address = Address::random_tagged("TcpSniRootInletListenProcessor");

        debug!("Binding TcpSniRootInletListenProcessor to {}", addr);
        let inner = match TcpListener::bind(addr).await {
            Ok(addr) => addr,
            Err(err) => {
                error!(%addr, %err, "could not bind to address");
                return Err(TransportError::from(err))?;
            }
        };
        let processor = Self::new(inner, registry);

        ctx.start_processor(processor_address.clone(), processor)?;

        Ok(processor_address)
    }
}

#[async_trait]
impl Processor for TcpSniRootInletListenProcessor {
    type Context = Context;

    #[instrument(skip_all, name = "TcpInletListenProcessor::process")]
    async fn process(&mut self, _ctx: &mut Self::Context) -> Result<bool> {
        let (mut stream, socket) = self.listener.accept().await.unwrap();

        info!("Accepted new root inlet connection");

        unsafe { self.buffer.set_len(BUFFER_LEN) };
        let size = stream
            .peek(&mut self.buffer)
            .await
            .map_err(TransportError::from)?;
        unsafe { self.buffer.set_len(size) };

        if self.buffer.starts_with(&PSQL_REQUEST_TLS_BIN) {
            info!("Received PSQL RequestTLS message.");
            let mut p = [0u8; PSQL_REQUEST_TLS_BIN.len()];
            stream
                .read_exact(&mut p)
                .await
                .map_err(TransportError::from)?;

            info!("Sending PSQL TLSSupported message.");
            stream
                .write_all(&PSQL_RESPONSE_TLS_BIN)
                .await
                .map_err(TransportError::from)?;

            unsafe { self.buffer.set_len(BUFFER_LEN) };
            let size = stream
                .peek(&mut self.buffer)
                .await
                .map_err(TransportError::from)?;
            unsafe { self.buffer.set_len(size) };
        }

        let sni = if let Some(sni) = extract_sni(&self.buffer) {
            info!("Root inlet connection SNI is {}", sni);
            sni
        } else {
            warn!("Root inlet connection SNI wasn't found");
            return Ok(true);
        };

        if let Some(sender) = self.registry.get_sni_listener(&sni) {
            info!("Found dedicated inlet for SNI {}", sni);
            tokio::spawn(async move {
                match sender.send((stream, socket)).await {
                    Ok(_) => {
                        info!("Sent new connection with SNI {}", sni);
                    }
                    Err(_) => {
                        warn!("Could not sent new connection with SNI {}", sni);
                    }
                }
            })
            .await
            .unwrap();
        } else {
            warn!("Dedicated inlet for SNI {} was not found", sni);
        }

        Ok(true)
    }
}

fn extract_sni(packet: &[u8]) -> Option<String> {
    let mut cursor = std::io::Cursor::new(packet);
    let mut acceptor = Acceptor::default();
    let _size = acceptor.read_tls(&mut cursor).ok()?;
    let accepted = acceptor.accept().ok()??;
    let hello = accepted.client_hello();
    hello.server_name().map(|s| s.to_string())
}
