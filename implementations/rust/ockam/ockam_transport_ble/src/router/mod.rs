mod handle;

use ockam_core::{
    async_trait,
    compat::{boxed::Box, collections::BTreeMap, vec::Vec},
    Any,
};
use ockam_core::{Address, Decodable, LocalMessage, Message, Result, Routed, Worker};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use serde::{Deserialize, Serialize};

pub(crate) use handle::BleRouterHandle;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Message)]
pub enum BleRouterMessage {
    /// Register a new client to this routing scope.
    Register {
        /// Specify an accept scope for this client.
        accepts: Vec<Address>,
        /// The clients own worker bus address.
        self_addr: Address,
    },
}

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
    main_addr: Address,
    api_addr: Address,
    map: BTreeMap<Address, Address>,
}

impl BleRouter {
    async fn create_self_handle(&self, ctx: &Context) -> Result<BleRouterHandle> {
        let handle_ctx = ctx.new_detached(Address::random_local()).await?;
        let handle = BleRouterHandle::new(handle_ctx, self.api_addr.clone());
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
        let next = match self.map.get(onward) {
            Some(addr) => addr.clone(),
            None => {
                error!("unknown route: {:?}", onward);
                return Err(TransportError::UnknownRoute.into());
            }
        };

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
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;
        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let msg_addr = msg.msg_addr();

        if msg_addr == self.main_addr {
            let msg = LocalMessage::decode(msg.payload())?;
            trace!("handle_message route: {:?}", msg.transport().onward_route);
            self.handle_route(ctx, msg).await?;
        } else if msg_addr == self.api_addr {
            let msg = BleRouterMessage::decode(msg.payload())?;
            match msg {
                BleRouterMessage::Register { accepts, self_addr } => {
                    trace!("handle_message register: {:?} => {:?}", accepts, self_addr);
                    self.handle_register(accepts, self_addr).await?;
                }
            };
        } else {
            return Err(TransportError::InvalidAddress.into());
        }

        Ok(())
    }
}

impl BleRouter {
    /// Create and register a new Ble router with the node context
    ///
    /// To also handle incoming connections, use
    /// [`BleRouter::bind`](BleRouter::bind)
    pub(crate) async fn register(ctx: &Context) -> Result<BleRouterHandle> {
        let main_addr = Address::random_local();
        let api_addr = Address::random_local();
        debug!("Registering new BleRouter with address {}", &main_addr);

        let child_ctx = ctx.new_detached(Address::random_local()).await?;
        let router = Self {
            _ctx: child_ctx,
            main_addr: main_addr.clone(),
            api_addr: api_addr.clone(),
            map: BTreeMap::new(),
        };

        let handle = router.create_self_handle(ctx).await?;

        trace!("Start Ble router for address = {:?}", main_addr.clone());
        ctx.start_worker(vec![main_addr.clone(), api_addr], router)
            .await?;

        trace!("Registering Ble router for type = {}", crate::BLE);
        ctx.register(crate::BLE, main_addr).await?;

        Ok(handle)
    }
}
