use crate::{TcpError, TcpPortalRecvProcessor, TcpPortalSendWorker};
use ockam_core::compat::net::SocketAddr;
use ockam_core::{Address, Result, Route};
use ockam_node::Context;
use tokio::net::TcpStream;
use tracing::{debug, trace};

/// Transmit and receive peers of a TCP connection
#[derive(Debug)]
pub(crate) struct PortalWorkerPair;

impl PortalWorkerPair {
    pub(crate) async fn new_inlet(
        ctx: &Context,
        stream: TcpStream,
        peer: SocketAddr,
        onward_route: Route,
    ) -> Result<()> {
        trace!("Creating new portal worker pair from stream");

        let tx_addr = Address::random(0);
        let tx_internal_addr = Address::random(0);
        let tx_remote_addr = Address::random(0);
        let rx_addr = Address::random(0);

        // Create two workers based on the split TCP I/O streams
        let (rx, tx) = stream.into_split();
        let sender = TcpPortalSendWorker::new(
            tx,
            peer,
            tx_internal_addr.clone(),
            tx_remote_addr.clone(),
            Some(onward_route),
        );
        let receiver = TcpPortalRecvProcessor::new(rx, tx_internal_addr.clone());

        // Derive local worker addresses, and start them
        ctx.start_worker(
            vec![tx_addr.clone(), tx_internal_addr, tx_remote_addr],
            sender,
        )
        .await?;
        ctx.start_processor(rx_addr.clone(), receiver).await?;

        // Return a handle to the worker pair
        Ok(())
    }

    pub(crate) async fn new_outlet(ctx: &Context, peer: SocketAddr) -> Result<Address> {
        debug!("Starting worker connection to remote {}", peer);

        // TODO: make i/o errors into ockam_error
        let stream = TcpStream::connect(peer).await.map_err(TcpError::from)?;

        trace!("Creating new portal worker pair from stream");

        let tx_internal_addr = Address::random(0);
        let tx_remote_addr = Address::random(0);
        let rx_addr = Address::random(0);

        // Create two workers based on the split TCP I/O streams
        let (rx, tx) = stream.into_split();
        let sender = TcpPortalSendWorker::new(
            tx,
            peer,
            tx_internal_addr.clone(),
            tx_remote_addr.clone(),
            None,
        );
        let receiver = TcpPortalRecvProcessor::new(rx, tx_internal_addr.clone());

        // Derive local worker addresses, and start them
        ctx.start_worker(vec![tx_internal_addr, tx_remote_addr.clone()], sender)
            .await?;
        ctx.start_processor(rx_addr.clone(), receiver).await?;

        // Return a handle to the worker pair
        Ok(tx_remote_addr)
    }
}
