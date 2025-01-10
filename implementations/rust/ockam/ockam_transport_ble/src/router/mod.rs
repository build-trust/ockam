mod handle;

use ockam_core::{
    async_trait,
    compat::{boxed::Box, collections::BTreeMap, vec::Vec},
    AllowAll, Any, Mailbox, Mailboxes,
};
use ockam_core::{Address, Decodable, LocalMessage, Message, Result, Routed, Worker};
use ockam_node::{Context, WorkerBuilder};
use ockam_transport_core::TransportError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
    fn create_self_handle(&self, ctx: &Context) -> Result<BleRouterHandle> {
        let handle_ctx = ctx.new_detached(
            Address::random_tagged("BleRouterHandle.try_clone.detached"),
            AllowAll,
            AllowAll,
        )?;
        let handle = BleRouterHandle::new(handle_ctx, self.api_addr.clone());
        Ok(handle)
    }

    async fn handle_register(&mut self, accepts: Vec<Address>, self_addr: Address) -> Result<()> {
        if let Some(f) = accepts.first().cloned() {
            debug!("BLE registration request: {} => {}", f, self_addr);
        } else {
            return Err(TransportError::InvalidAddress(
                accepts
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
                    .to_string(),
            ))?;
        }

        for accept in &accepts {
            if self.map.contains_key(accept) {
                return Err(TransportError::AlreadyConnected)?;
            }
        }

        for accept in accepts {
            self.map.insert(accept.clone(), self_addr.clone());
        }

        Ok(())
    }

    async fn handle_route(&mut self, ctx: &Context, msg: LocalMessage) -> Result<()> {
        debug!("Ble route request: {:?}", msg.onward_route());

        // Get the next hop
        let onward = msg.next_on_onward_route()?;

        // Look up the connection worker responsible
        let next = match self.map.get(onward) {
            Some(addr) => addr.clone(),
            None => {
                error!("unknown route: {:?}", onward);
                return Err(TransportError::UnknownRoute)?;
            }
        };

        // Modify the transport message route
        let msg = msg.replace_front_onward_route(next.clone())?;

        // Send the transport message to the connection worker
        ctx.send(next.clone(), msg).await?;

        Ok(())
    }
}

#[async_trait]
impl Worker for BleRouter {
    type Context = Context;
    type Message = Any;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let msg_addr = msg.msg_addr();

        if msg_addr == self.main_addr {
            let msg = LocalMessage::decode(msg.payload())?;
            trace!("handle_message route: {:?}", msg.onward_route());
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
            return Err(TransportError::InvalidAddress(msg_addr.to_string()))?;
        }

        Ok(())
    }
}

impl BleRouter {
    /// Create and register a new Ble router with the node context
    ///
    /// To also handle incoming connections, use
    /// [`BleRouter::bind`](BleRouter::bind)
    pub(crate) fn register(ctx: &Context) -> Result<BleRouterHandle> {
        let main_addr = Address::random_tagged("BleRouter.main_addr");
        let api_addr = Address::random_tagged("BleRouter.api_addr");
        debug!("Registering new BleRouter with address {}", &main_addr);

        let child_ctx = ctx.new_detached(
            Address::random_tagged("BleRouter.detached_child"),
            AllowAll,
            AllowAll,
        )?;
        let router = Self {
            _ctx: child_ctx,
            main_addr: main_addr.clone(),
            api_addr: api_addr.clone(),
            map: BTreeMap::new(),
        };

        let handle = router.create_self_handle(ctx)?;

        trace!("Start Ble router for address = {:?}", main_addr.clone());

        // TODO: @ac
        let mailboxes = Mailboxes::new(
            Mailbox::new(
                main_addr.clone(),
                None,
                Arc::new(AllowAll),
                Arc::new(AllowAll),
            ),
            vec![Mailbox::new(
                api_addr,
                None,
                Arc::new(AllowAll),
                Arc::new(AllowAll),
            )],
        );
        WorkerBuilder::new(router)
            .with_mailboxes(mailboxes)
            .start(ctx)?;

        trace!("Registering Ble router for type = {}", crate::BLE);
        ctx.register(crate::BLE, main_addr)?;

        Ok(handle)
    }
}
