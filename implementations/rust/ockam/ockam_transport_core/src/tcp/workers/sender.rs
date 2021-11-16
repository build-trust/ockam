use super::TcpRecvProcessor;
use crate::tcp::traits::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use crate::tcp::traits::{IntoSplit, TcpStreamConnector};
use crate::TransportError;
use core::fmt::Display;
use core::ops::Deref;
use core::time::Duration;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::{
    async_trait, route, Address, Any, Decodable, Encodable, LocalMessage, Result, Routed,
    TransportMessage, Worker,
};
use ockam_node::{Context, Heartbeat};
use tracing::{debug, trace, warn};

/// Transmit and receive peers of a TCP connection
#[derive(Debug)]
pub struct WorkerPair<T, U> {
    hostnames: T,
    endpoint: U,
    tx_addr: Address,
}

impl<T, U, V> WorkerPair<T, U>
where
    T: Deref<Target = [V]>,
    U: Clone,
{
    /// Returns the hostnames of the remote peer associated with the worker pair.
    pub fn hostnames(&self) -> &[V] {
        &self.hostnames
    }

    /// Returns the socket address of the peer to the worker pair.
    pub fn peer(&self) -> U {
        self.endpoint.clone()
    }

    /// Returns the external address of the pair.
    pub fn tx_addr(&self) -> Address {
        self.tx_addr.clone()
    }
}

/// A TCP sending message worker
///
/// Create this worker type by calling
/// [TcpSendWorker::start_pair]
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub struct TcpSendWorker<T, U, V, W> {
    rx: Option<T>,
    tx: Option<U>,
    stream_connector: V,
    endpoint: W,
    internal_addr: Address,
    heartbeat: Heartbeat<Vec<u8>>,
    heartbeat_interval: Option<Duration>,
    cluster_name: &'static str,
}

impl<T, U, V, W> TcpSendWorker<T, U, V, W>
where
    W: Display + Clone + Send + Sync + 'static,
    V: TcpStreamConnector<W> + Send + Sync + 'static,
    V::Stream: IntoSplit<ReadHalf = T, WriteHalf = U>,
    T: AsyncRead + Send + Unpin + 'static,
    U: AsyncWrite + Send + Unpin + 'static,
{
    fn new<Y>(
        stream: Option<Y>,
        stream_connector: V,
        endpoint: W,
        internal_addr: Address,
        cluster_name: &'static str,
        heartbeat: Heartbeat<Vec<u8>>,
    ) -> Self
    where
        Y: IntoSplit<ReadHalf = T, WriteHalf = U>,
    {
        let (rx, tx) = match stream {
            Some(s) => {
                let (rx, tx) = s.into_split();
                (Some(rx), Some(tx))
            }
            None => (None, None),
        };

        Self {
            rx,
            tx,
            stream_connector,
            endpoint,
            internal_addr,
            heartbeat,
            heartbeat_interval: Some(Duration::from_secs(5 * 60)),
            cluster_name,
        }
    }

    /// Starts a pair of recv/send workers to process messages from `peer`.
    ///
    /// It returns a [WorkerPair] that holds the information of the pair of workers.
    pub async fn start_pair<Y, Z>(
        ctx: &Context,
        stream: Option<Y>,
        stream_connector: V,
        endpoint: W,
        hostnames: Z,
        cluster_name: &'static str,
    ) -> Result<WorkerPair<Z, W>>
    where
        Y: IntoSplit<ReadHalf = T, WriteHalf = U>,
    {
        trace!("Creating new TCP worker pair");

        let tx_addr = Address::random(0);
        let internal_addr = Address::random(0);
        let sender = TcpSendWorker::new(
            stream,
            stream_connector,
            endpoint.clone(),
            internal_addr.clone(),
            cluster_name,
            Heartbeat::create(ctx, internal_addr.clone(), vec![]).await?,
        );

        // TODO allocation
        // Here the allocation is due to AdressSet: From<[Adress; 2]> or something similar not being implemented
        ctx.start_worker(vec![tx_addr.clone(), internal_addr], sender)
            .await?;

        // Return a handle to the worker pair
        Ok(WorkerPair {
            hostnames,
            endpoint,
            tx_addr,
        })
    }

    async fn schedule_heartbeat(&mut self) -> Result<()> {
        let heartbeat_interval;
        if let Some(hi) = &self.heartbeat_interval {
            heartbeat_interval = *hi;
        } else {
            return Ok(());
        }

        self.heartbeat.schedule(heartbeat_interval).await
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
impl<T, U, V, W, X> Worker for TcpSendWorker<T, U, V, W>
where
    V: TcpStreamConnector<W, Stream = X> + Send + Sync + 'static,
    X: IntoSplit<ReadHalf = T, WriteHalf = U>,
    T: Send + AsyncRead + Unpin + 'static,
    U: Send + AsyncWrite + Unpin + 'static,
    W: Clone + Send + Sync + core::fmt::Display + 'static,
{
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.set_cluster(self.cluster_name).await?;

        if self.tx.is_none() {
            let (rx, tx) = self
                .stream_connector
                .connect(self.endpoint.clone())
                .await?
                .into_split();
            self.tx = Some(tx);
            self.rx = Some(rx);
        }

        if let Some(rx) = self.rx.take() {
            let rx_addr = Address::random(0);
            let receiver = TcpRecvProcessor::new(
                rx,
                format!("{}#{}", crate::TCP, self.endpoint).into(),
                self.cluster_name,
            );
            ctx.start_processor(rx_addr.clone(), receiver).await?;
        } else {
            return Err(TransportError::GenericIo.into());
        }

        self.schedule_heartbeat().await?;

        Ok(())
    }

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        self.heartbeat.cancel();

        let tx;
        if let Some(t) = &mut self.tx {
            tx = t;
        } else {
            return Err(TransportError::PeerNotFound.into());
        }

        let recipient = msg.msg_addr();
        if recipient == self.internal_addr {
            let msg = TransportMessage::v1(route![], route![], vec![]);
            let msg = prepare_message(msg)?;
            // Sending empty heartbeat
            if tx.write_all(&msg).await.is_err() {
                warn!("Failed to send heartbeat to peer {}", self.endpoint);
                ctx.stop_worker(ctx.address()).await?;

                return Ok(());
            }

            debug!("Sent heartbeat to peer {}", self.endpoint);
        } else {
            let mut msg = LocalMessage::decode(msg.payload())?.into_transport_message();
            // Remove our own address from the route so the other end
            // knows what to do with the incoming message
            msg.onward_route.step()?;
            // Create a message buffer with pre-pended length
            let msg = prepare_message(msg)?;

            if tx.write_all(msg.as_slice()).await.is_err() {
                warn!("Failed to send message to peer {}", self.endpoint);
                ctx.stop_worker(ctx.address()).await?;

                return Ok(());
            }
        }

        self.schedule_heartbeat().await?;

        Ok(())
    }
}
