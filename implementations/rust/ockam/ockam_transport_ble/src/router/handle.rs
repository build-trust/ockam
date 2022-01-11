use crate::{
    driver::{BleClientDriver, BleServerDriver, BleStreamDriver},
    workers::{BleListenProcessor, BleSendWorker, WorkerPair},
    BleAddr, BleClient, BleServer,
};

use ockam_core::{
    async_trait,
    compat::{boxed::Box, string::String, vec::Vec},
};
use ockam_core::{Address, AsyncTryClone, Result, RouterMessage};
use ockam_node::Context;
use ockam_transport_core::TransportError;

/// A handle to connect to a BleRouter
///
/// Dropping this handle is harmless.
pub(crate) struct BleRouterHandle {
    ctx: Context,
    addr: Address,
}

#[async_trait]
impl AsyncTryClone for BleRouterHandle {
    async fn async_try_clone(&self) -> Result<Self> {
        let child_ctx = self.ctx.new_context(Address::random(0)).await?;
        Ok(Self::new(child_ctx, self.addr.clone()))
    }
}

impl BleRouterHandle {
    pub(crate) fn new(ctx: Context, addr: Address) -> Self {
        BleRouterHandle { ctx, addr }
    }
}

impl BleRouterHandle {
    /// Register a new connection worker with this router
    pub async fn register(&self, pair: &WorkerPair) -> Result<()> {
        let ble_address: Address = format!("{}#{}", crate::BLE, pair.peer()).into();
        let mut accepts = vec![ble_address];
        accepts.extend(
            pair.servicenames()
                .iter()
                .map(|x| Address::from_string(format!("{}#{}", crate::BLE, x))),
        );
        let self_addr = pair.tx_addr();

        trace!("BleRouterHandle accepts: {:?} -> {:?}", accepts, self_addr);

        self.ctx
            .send(
                self.addr.clone(),
                RouterMessage::Register { accepts, self_addr },
            )
            .await
    }

    /// Bind an incoming connection listener for this router
    pub async fn bind<A: BleServerDriver + BleStreamDriver + Send + 'static, S: Into<BleAddr>>(
        &self,
        ble_server: BleServer<A>,
        addr: S,
    ) -> Result<()> {
        let ble_addr = addr.into();
        BleListenProcessor::start(
            ble_server,
            &self.ctx,
            self.async_try_clone().await?,
            ble_addr,
        )
        .await
    }

    pub(crate) fn resolve_peer(peer: impl Into<String>) -> Result<(BleAddr, Vec<String>)> {
        let peer_str = peer.into();
        let peer_addr;
        let servicenames;

        // Try to parse as BleAddr
        if let Ok(p) = crate::parse_ble_addr(peer_str) {
            peer_addr = p;
            servicenames = vec![];
        } else {
            return Err(TransportError::InvalidAddress.into());
        }

        Ok((peer_addr, servicenames))
    }

    /// Establish an outgoing BLE connection on an existing transport
    pub async fn connect<A: BleClientDriver + BleStreamDriver + Send + 'static, S: AsRef<str>>(
        &self,
        mut ble_client: BleClient<A>,
        peer: S,
    ) -> Result<()> {
        let (peer_addr, servicenames) = Self::resolve_peer(peer.as_ref())?;

        debug!("scanning all available adapters");
        ble_client.scan(&peer_addr).await?;

        debug!("connecting to server peripheral");
        ble_client.connect().await?;

        let stream = crate::driver::AsyncStream::with_ble_device(ble_client);
        let pair = BleSendWorker::start_pair(&self.ctx, stream, peer_addr, servicenames).await?;

        self.register(&pair).await?;

        Ok(())
    }
}
