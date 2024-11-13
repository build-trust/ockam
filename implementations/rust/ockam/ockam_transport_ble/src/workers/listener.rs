use ockam_core::compat::boxed::Box;
use ockam_core::{async_trait, Address, AllowAll, Processor, Result};
use ockam_node::Context;

use crate::driver::AsyncStream;
use crate::driver::{BleEvent, BleServer};
use crate::driver::{BleServerDriver, BleStreamDriver};
use crate::router::BleRouterHandle;
use crate::workers::sender::BleSendWorker;
use crate::BleAddr;

/// BleListenProcessor
pub struct BleListenProcessor<A> {
    inner: Option<BleServer<A>>,
    router_handle: BleRouterHandle,
}

impl<A> BleListenProcessor<A>
where
    A: BleServerDriver + BleStreamDriver + Send + 'static,
{
    pub(crate) async fn start(
        mut ble_server: BleServer<A>,
        ctx: &Context,
        router_handle: BleRouterHandle,
        addr: BleAddr,
    ) -> Result<()> {
        debug!("BleRouterHandle::bind binding BleServer to: {}", addr);
        ble_server.bind(&addr).await?;

        let processor = Self {
            inner: Some(ble_server),
            router_handle,
        };
        let waddr = Address::random_local();

        debug!(
            "BleListenProcessor::start Starting processor with address: {:?}",
            waddr
        );
        ctx.start_processor_with_access_control(
            waddr, processor, AllowAll, // FIXME: @ac
            AllowAll, // FIXME: @ac
        )?;

        Ok(())
    }
}

#[async_trait]
impl<A> Processor for BleListenProcessor<A>
where
    A: BleServerDriver + BleStreamDriver + Send + 'static,
{
    type Context = Context;

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        if self.inner.is_none() {
            return Ok(true);
        }

        // Wait for an incoming connection from a BleClient
        let mut buffer = [0_u8; 64];
        let result = self.inner.as_mut().unwrap().poll(&mut buffer).await;

        // TODO some BLE devices can pair with multiple clients
        // TODO some targets have multiple BLE devices

        if let Ok(BleEvent::ConnectionComplete) = result {
            trace!("Spawning WorkerPair for BleServer");
            // Spawn a WorkerPair for it
            let ble_server = self.inner.take().unwrap();
            let stream = AsyncStream::with_ble_device(ble_server);
            let pair = BleSendWorker::start_pair(
                ctx,
                stream,
                // TODO resolve connecting BleClient's addresses
                crate::parse_ble_addr("ble_client_addr").unwrap(),
                vec![],
            )?;

            // Register the connection with the local BleRouter
            trace!("Registering WorkerPair tx stream with BleRouterHandle");
            self.router_handle.register(&pair).await?;
        } else if let Ok(BleEvent::None) = result {
            // TODO sleep
        } else {
            error!("Unhandled event: {:?}", result);
        }

        Ok(true)
    }
}
