use crate::{
    driver::{BleClientDriver, BleServerDriver, BleStreamDriver},
    workers::{BleListenProcessor, BleSendWorker, WorkerPair},
    BleAddr, BleClient, BleServer,
};

use crate::router::BleRouterMessage;
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::{Address, Result, TryClone};
use ockam_node::Context;
use ockam_transport_core::TransportError;

/// A handle to connect to a BleRouter
///
/// Dropping this handle is harmless.
#[derive(TryClone)]
#[try_clone(crate = "ockam_core")]
pub(crate) struct BleRouterHandle {
    ctx: Context,
    api_addr: Address,
}

impl BleRouterHandle {
    pub(crate) fn new(ctx: Context, api_addr: Address) -> Self {
        BleRouterHandle { ctx, api_addr }
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
                self.api_addr.clone(),
                BleRouterMessage::Register { accepts, self_addr },
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
        BleListenProcessor::start(ble_server, &self.ctx, self.try_clone()?, ble_addr).await
    }

    // TODO: Remove in favor of `ockam_node::compat::asynchronous::resolve_peer`
    pub(crate) fn resolve_peer(peer: impl Into<String>) -> Result<(BleAddr, Vec<String>)> {
        let peer_str = peer.into();
        let peer_addr;
        let servicenames;

        // Try to parse as BleAddr
        if let Ok(p) = crate::parse_ble_addr(peer_str.clone()) {
            peer_addr = p;
            servicenames = vec![];
        } else {
            return Err(TransportError::InvalidAddress(peer_str))?;
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
        let pair = BleSendWorker::start_pair(&self.ctx, stream, peer_addr, servicenames)?;

        self.register(&pair).await?;

        Ok(())
    }
}
