use crate::{
    atomic::{self, ArcBool},
    TcpRecvWorker, TcpSendWorker,
};
use ockam_core::{Address, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tracing::{debug, trace};

/// Transmit and receive peers of a TCP connection
#[derive(Debug)]
pub(crate) struct WorkerPair {
    hostnames: Vec<String>,
    peer: SocketAddr,
    tx_addr: Address,
    rx_addr: Address,
    run: ArcBool,
}

impl WorkerPair {
    pub fn hostnames(&self) -> &Vec<String> {
        &self.hostnames
    }
    pub fn peer(&self) -> SocketAddr {
        self.peer
    }
    pub fn tx_addr(&self) -> Address {
        self.tx_addr.clone()
    }
}

impl WorkerPair {
    pub(crate) async fn new_with_stream(
        ctx: &Context,
        stream: TcpStream,
        peer: SocketAddr,
        hostnames: Vec<String>,
    ) -> Result<Self> {
        trace!("Creating new worker pair from stream");

        // TODO: Should we also check for TcpTransport run flag?
        let run = atomic::new(true);

        let tx_addr = Address::random(0);
        let rx_addr = Address::random(0);

        // Create two workers based on the split TCP I/O streams
        let (rx, tx) = stream.into_split();
        let sender = TcpSendWorker::new(tx, peer);
        let receiver =
            TcpRecvWorker::new(rx, run.clone(), format!("{}#{}", crate::TCP, peer).into());

        // Derive local worker addresses, and start them
        ctx.start_worker(tx_addr.clone(), sender).await?;
        ctx.start_worker(rx_addr.clone(), receiver).await?;

        // Return a handle to the worker pair
        Ok(WorkerPair {
            hostnames,
            peer,
            rx_addr,
            tx_addr,
            run,
        })
    }

    pub(crate) async fn start(
        ctx: &Context,
        peer: SocketAddr,
        hostnames: Vec<String>,
    ) -> Result<Self> {
        debug!("Starting worker connection to remote {}", peer);

        // TODO: make i/o errors into ockam_error
        let stream = TcpStream::connect(peer)
            .await
            .map_err(TransportError::from)?;
        Self::new_with_stream(ctx, stream, peer, hostnames).await
    }
}
