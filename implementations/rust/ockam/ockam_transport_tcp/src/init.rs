use crate::{
    atomic::{self, ArcBool},
    TcpError, TcpRecvWorker, TcpRouterHandle, TcpSendWorker,
};
use ockam_core::{Address, Result};
use ockam_node::Context;
use std::net::SocketAddr;
use tokio::net::TcpStream;

/// Transmit and receive peers of a TCP connection
#[derive(Debug)]
pub struct WorkerPair {
    pub(crate) peer: SocketAddr,
    pub(crate) tx_addr: Address,
    pub(crate) rx_addr: Address,
    run: ArcBool,
}

impl WorkerPair {
    /// Stop the worker pair
    pub async fn stop(self, ctx: &Context) -> Result<()> {
        ctx.stop_worker(self.tx_addr).await?;
        ctx.stop_worker(self.rx_addr).await?;
        atomic::stop(&self.run);
        Ok(())
    }

    fn from_peer(peer: SocketAddr) -> Self {
        Self {
            peer,
            tx_addr: Address::random(0),
            rx_addr: Address::random(0),
            run: atomic::new(true),
        }
    }
}

impl WorkerPair {
    pub(crate) async fn with_stream(
        ctx: &Context,
        stream: TcpStream,
        peer: SocketAddr,
    ) -> Result<Self> {
        let WorkerPair {
            peer,
            rx_addr,
            tx_addr,
            run,
        } = WorkerPair::from_peer(peer);

        trace!("Creating new worker pair from stream");

        // Create two workers based on the split TCP I/O streams
        let (rx, tx) = stream.into_split();
        let sender = TcpSendWorker { tx, peer };
        let receiver = TcpRecvWorker {
            rx,
            run: run.clone(),
            peer_addr: format!("{}#{}", crate::TCP, peer).into(),
        };

        // Derive local worker addresses, and start them
        ctx.start_worker(tx_addr.clone(), sender).await?;
        ctx.start_worker(rx_addr.clone(), receiver).await?;

        // Return a handle to the worker pair
        Ok(WorkerPair {
            peer,
            rx_addr,
            tx_addr,
            run,
        })
    }

    async fn start(ctx: &Context, peer: SocketAddr) -> Result<Self> {
        debug!("Starting worker connection to remote {}", peer);

        // TODO: make i/o errors into ockam_error
        let stream = TcpStream::connect(peer).await.map_err(TcpError::from)?;
        Self::with_stream(ctx, stream, peer).await
    }
}

/// Start a new pair of TCP connection workers
///
/// One worker handles outgoing messages, while another handles
/// incoming messages.  The local worker address is chosen based on
/// the peer the worker is meant to be connected to.
pub async fn start_connection<P>(
    ctx: &Context,
    router: &TcpRouterHandle<'_>,
    peer: P,
) -> Result<WorkerPair>
where
    P: Into<SocketAddr>,
{
    let peer = peer.into();
    let pair = WorkerPair::start(ctx, peer).await?;
    router.register(&pair).await?;
    Ok(pair)
}
