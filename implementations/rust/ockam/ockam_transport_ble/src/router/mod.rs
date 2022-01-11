mod handle;

use ockam::LocalMessage;
use ockam_core::{
    async_trait,
    compat::{boxed::Box, collections::BTreeMap, vec::Vec},
};
use ockam_core::{Address, Result, Routed, RouterMessage, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;

pub(crate) use handle::BleRouterHandle;

/// A Bluetooth Low Energy address router and connection listener
///
/// In order to create new BLE connection workers you need a router to
/// map remote addresses of `type = BLE` to worker addresses.  This type
/// facilitates this.
///
/// Optionally you can also start listening for incoming connections
/// if the local node is part of a server architecture.
pub struct BleRouter {
    _ctx: Context,
    addr: Address,
    map: BTreeMap<Address, Address>,
}

impl BleRouter {
    async fn create_self_handle(&self, ctx: &Context) -> Result<BleRouterHandle> {
        let handle_ctx = ctx.new_context(Address::random(0)).await?;
        let handle = BleRouterHandle::new(handle_ctx, self.addr.clone());
        Ok(handle)
    }

    async fn handle_register(&mut self, accepts: Vec<Address>, self_addr: Address) -> Result<()> {
        if let Some(f) = accepts.first().cloned() {
            debug!("BLE registration request: {} => {}", f, self_addr);
        } else {
            return Err(TransportError::InvalidAddress.into());
        }

        for accept in &accepts {
            if self.map.contains_key(accept) {
                return Err(TransportError::AlreadyConnected.into());
            }
        }

        for accept in accepts {
            self.map.insert(accept.clone(), self_addr.clone());
        }

        Ok(())
    }

    async fn handle_route(&mut self, ctx: &Context, mut msg: LocalMessage) -> Result<()> {
        debug!("Ble route request: {:?}", msg.transport().onward_route);

        // Get the next hop
        let onward = msg.transport().onward_route.next()?;

        // Look up the connection worker responsible
        let next;
        if let Some(addr) = self.map.get(onward) {
            next = addr.clone();
        } else {
            error!("unknown route: {:?}", onward);
            return Err(TransportError::UnknownRoute.into());
        }

        let _ = msg.transport_mut().onward_route.step()?;

        // Modify the transport message route
        msg.transport_mut()
            .onward_route
            .modify()
            .prepend(next.clone());

        // Send the transport message to the connection worker
        ctx.send(next.clone(), msg).await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for BleRouter {
    type Context = Context;
    type Message = RouterMessage;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        debug!("Registering Ble router for type = {}", crate::BLE);
        ctx.register(crate::BLE, ctx.address()).await?;
        ctx.set_cluster(crate::CLUSTER_NAME).await?;
        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<RouterMessage>,
    ) -> Result<()> {
        let msg = msg.body();
        use RouterMessage::*;
        match msg {
            Route(msg) => {
                trace!("handle_message route: {:?}", msg.transport().onward_route);
                self.handle_route(ctx, msg).await?;
            }
            Register { accepts, self_addr } => {
                trace!("handle_message register: {:?} => {:?}", accepts, self_addr);
                self.handle_register(accepts, self_addr).await?;
            }
        };

        Ok(())
    }
}

impl BleRouter {
    /// Create and register a new Ble router with the node context
    ///
    /// To also handle incoming connections, use
    /// [`BleRouter::bind`](BleRouter::bind)
    pub(crate) async fn register(ctx: &Context) -> Result<BleRouterHandle> {
        let addr = Address::random(0);
        debug!("Registering new BleRouter with address {}", &addr);

        let child_ctx = ctx.new_context(Address::random(0)).await?;
        let router = Self {
            _ctx: child_ctx,
            addr: addr.clone(),
            map: BTreeMap::new(),
        };

        let handle = router.create_self_handle(ctx).await?;

        trace!("BleRouter start_worker({:?})", addr.clone());
        ctx.start_worker(addr.clone(), router).await?;

        Ok(handle)
    }
}
