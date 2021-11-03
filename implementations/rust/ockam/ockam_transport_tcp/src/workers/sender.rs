use crate::TcpRecvProcessor;
use ockam_core::async_trait;
use ockam_core::{Address, Encodable, Result, Routed, TransportMessage, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tracing::{trace, warn};

/// Transmit and receive peers of a TCP connection
#[derive(Debug)]
pub(crate) struct WorkerPair {
    hostnames: Vec<String>,
    peer: SocketAddr,
    tx_addr: Address,
}

impl WorkerPair {
    pub fn hostnames(&self) -> &[String] {
        &self.hostnames
    }
    pub fn peer(&self) -> SocketAddr {
        self.peer
    }
    pub fn tx_addr(&self) -> Address {
        self.tx_addr.clone()
    }
}

/// A TCP sending message worker
///
/// Create this worker type by calling
/// [`start_tcp_worker`](crate::start_tcp_worker)!
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub(crate) struct TcpSendWorker {
    rx: Option<OwnedReadHalf>,
    tx: Option<OwnedWriteHalf>,
    peer: SocketAddr,
}

impl TcpSendWorker {
    fn new(stream: Option<TcpStream>, peer: SocketAddr) -> Self {
        let (rx, tx) = match stream {
            Some(s) => {
                let (rx, tx) = s.into_split();
                (Some(rx), Some(tx))
            }
            None => (None, None),
        };

        Self { rx, tx, peer }
    }

    pub(crate) async fn start_pair(
        ctx: &Context,
        stream: Option<TcpStream>,
        peer: SocketAddr,
        hostnames: Vec<String>,
    ) -> Result<WorkerPair> {
        trace!("Creating new TCP worker pair");

        let tx_addr = Address::random(0);
        let sender = TcpSendWorker::new(stream, peer);

        ctx.start_worker(tx_addr.clone(), sender).await?;

        // Return a handle to the worker pair
        Ok(WorkerPair {
            hostnames,
            peer,
            tx_addr,
        })
    }
}

fn prepare_message(msg: TransportMessage) -> Result<Vec<u8>> {
    let mut msg_buf = msg.encode().map_err(|_| TransportError::SendBadMessage)?;

    // Create a buffer that includes the message length in big endian
    let mut len = (msg_buf.len() as u16).to_be_bytes().to_vec();

    // Fun fact: reversing a vector in place, appending the length,
    // and then reversing it again is faster for large message sizes
    // than adding the large chunk of data.
    //
    // https://play.rust-lang.org/?version=stable&mode=release&edition=2018&gist=8669a640004ac85c7be38b19e3e73dcb
    msg_buf.reverse();
    len.reverse();
    msg_buf.append(&mut len);
    msg_buf.reverse();

    Ok(msg_buf)
}

#[async_trait]
impl Worker for TcpSendWorker {
    type Context = Context;
    type Message = TransportMessage;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;

        if self.tx.is_none() {
            let (rx, tx) = TcpStream::connect(self.peer)
                .await
                .map_err(TransportError::from)?
                .into_split();
            self.tx = Some(tx);
            self.rx = Some(rx);
        }
        if let Some(rx) = self.rx.take() {
            let rx_addr = Address::random(0);
            let receiver =
                TcpRecvProcessor::new(rx, format!("{}#{}", crate::TCP, self.peer).into());
            ctx.start_processor(rx_addr.clone(), receiver).await?;
        } else {
            return Err(TransportError::GenericIo.into());
        }

        Ok(())
    }

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        mut msg: Routed<TransportMessage>,
    ) -> Result<()> {
        let tx;
        if let Some(t) = &mut self.tx {
            tx = t;
        } else {
            return Err(TransportError::PeerNotFound.into());
        }
        // Remove our own address from the route so the other end
        // knows what to do with the incoming message
        msg.onward_route.step()?;

        // Create a message buffer with pre-pended length
        let msg = prepare_message(msg.body())?;

        if tx.write(msg.as_slice()).await.is_err() {
            warn!("Failed to send message to peer {}", self.peer);
            ctx.stop_worker(ctx.address()).await?;
        }

        Ok(())
    }
}
