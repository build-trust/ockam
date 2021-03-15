use crate::{TcpRecvWorker, TcpSendWorker};
use ockam::{Address, Context, Result};
use std::{
    net::SocketAddr,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::net::TcpStream;

pub struct WorkerPair {
    tx_addr: Address,
    rx_addr: Address,
    run: Arc<AtomicBool>,
}

impl WorkerPair {
    /// Stop the worker pair
    pub async fn stop(self, ctx: &Context) -> Result<()> {
        ctx.stop_worker(self.tx_addr).await?;
        ctx.stop_worker(self.rx_addr).await?;
        Ok(())
    }

    fn from_peer(addr: &SocketAddr) -> Self {
        Self {
            tx_addr: format!("{}_tx", addr).into(),
            rx_addr: format!("{}_rx", addr).into(),
            run: Arc::new(AtomicBool::new(true)),
        }
    }
}

/// Start a new pair of TCP connection workers
///
/// One worker handles outgoing messages, while another handles
/// incoming messages.  The local worker address is chosen based on
/// the peer the worker is meant to be connected to.
pub async fn start_tcp_worker<P>(ctx: &Context, peer: P) -> Result<WorkerPair>
where
    P: Into<SocketAddr>,
{
    let peer = peer.into();

    // TODO: make i/o errors into ockam_error
    let stream = TcpStream::connect(peer.clone()).await.unwrap();

    let WorkerPair {
        rx_addr,
        tx_addr,
        run,
    } = WorkerPair::from_peer(&peer);

    // Create two workers based on the split TCP I/O streams
    let (rx, tx) = stream.into_split();
    let sender = TcpSendWorker { tx };
    let receiver = TcpRecvWorker {
        rx,
        run: Arc::clone(&run),
    };

    // Derive local worker addresses, and start them
    ctx.start_worker(tx_addr.clone(), sender).await?;
    ctx.start_worker(rx_addr.clone(), receiver).await?;

    // Return a handle to the worker pair
    Ok(WorkerPair {
        rx_addr,
        tx_addr,
        run,
    })
}
