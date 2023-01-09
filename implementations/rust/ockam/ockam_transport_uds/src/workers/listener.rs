use std::os::unix::net::SocketAddr;

use ockam_core::{
    async_trait, compat::sync::Arc, Address, AllowSourceAddress, AsyncTryClone, DenyAll, Mailbox,
    Mailboxes, Processor, Result,
};

use ockam_node::{Context, ProcessorBuilder, WorkerBuilder};
use ockam_transport_core::TransportError;
use tokio::net::UnixListener;
use tracing::{debug, error, trace};

use crate::{router::UdsRouterHandle, std_socket_addr_from_tokio, workers::UdsSendWorker};

/// A UDS Listener Processor
///
/// UDS Listen processors are created by `UdsTransport`
/// after a call is made to [`UdsTransport::listen`](crate::transport::UdsTransport)
pub(crate) struct UdsListenProcessor {
    inner: UnixListener,
    router_handle: UdsRouterHandle,
}

impl UdsListenProcessor {
    /// Binds a UDS socket at the given [`SocketAddr`]
    ///
    /// Starts a [`Processor`] which listens for incoming connections to accept.
    pub(crate) async fn start(
        ctx: &Context,
        router_handle: UdsRouterHandle,
        addr: SocketAddr,
    ) -> Result<SocketAddr> {
        let path = match addr.as_pathname() {
            Some(p) => p,
            None => {
                error!("Error binding to socket address {:?}", addr);
                return Err(TransportError::InvalidAddress.into());
            }
        };
        debug!("Binding UnixListener to {}", path.display());
        let inner = UnixListener::bind(path).map_err(TransportError::from)?;

        let tokio_sock_addr = inner.local_addr().map_err(TransportError::from)?;

        let std_sock_addr = std_socket_addr_from_tokio(&tokio_sock_addr)?;

        let processor = Self {
            inner,
            router_handle,
        };

        let mailbox = Mailbox::deny_all(Address::random_tagged("UdsListenProcessor"));
        ProcessorBuilder::with_mailboxes(Mailboxes::new(mailbox, vec![]), processor)
            .start(ctx)
            .await?;

        Ok(std_sock_addr)
    }
}

#[async_trait]
impl Processor for UdsListenProcessor {
    type Context = Context;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await
    }

    /// Listen for and accept incoming UDS connections.
    ///
    /// Register the peers socket address, and create a worker to communicate with the peer.
    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        debug!("Waiting for incoming UDS connection...");

        // Wait for an incoming connection
        let (stream, _peer) = self.inner.accept().await.map_err(TransportError::from)?;
        debug!("UDS connection accepted");

        // Create a connection working
        let handle_clone = self.router_handle.async_try_clone().await?;
        let local_addr = stream.local_addr().map_err(TransportError::from)?;
        let std_sock_addr = std_socket_addr_from_tokio(&local_addr)?;
        let (send_worker, pair) =
            UdsSendWorker::new_pair(handle_clone, Some(stream), std_sock_addr, vec![]).await?;

        self.router_handle.register(&pair).await?;
        debug!("UDS connection registered");

        trace! {
            tx_addr = %pair.tx_addr(),
            int_addr = %send_worker.internal_addr(),
            "starting UDS connection worker"
        };

        let tx_mailbox = Mailbox::new(
            pair.tx_addr(),
            Arc::new(AllowSourceAddress(self.router_handle.main_addr().clone())),
            Arc::new(DenyAll),
        );

        let internal_mailbox = Mailbox::new(
            send_worker.internal_addr().clone(),
            Arc::new(AllowSourceAddress(send_worker.rx_addr().clone())),
            Arc::new(DenyAll),
        );

        let mailboxes = Mailboxes::new(tx_mailbox, vec![internal_mailbox]);
        WorkerBuilder::with_mailboxes(mailboxes, send_worker)
            .start(ctx)
            .await?;

        Ok(true)
    }
}
