use crate::{async_worker, Context, OckamError};
use ockam_core::lib::net::SocketAddr;
use ockam_core::lib::PhantomData;
use ockam_core::{Address, Message, Result, Route, Routed, TransportMessage, Worker};
use rand::random;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct RemoteMailboxInfo {
    forwarding_route: Route,
    remote_address: String,
    worker_address: Address,
}

impl RemoteMailboxInfo {
    pub fn forwarding_route(&self) -> &Route {
        &self.forwarding_route
    }
    pub fn remote_address(&self) -> &str {
        &self.remote_address
    }
    pub fn worker_address(&self) -> &Address {
        &self.worker_address
    }
}

/// This Worker is responsible for registering on Ockam Hub and forwarding message with type T to
/// local Worker
pub struct RemoteMailbox<T: Message> {
    route: Route,
    destination: Route,
    callback_address: Address,
    phantom_t: PhantomData<T>,
}

impl<T: Message> RemoteMailbox<T> {
    fn new(hub_addr: SocketAddr, destination: Address, callback_address: Address) -> Self {
        let route = Route::new()
            .append(format!("1#{}", hub_addr))
            .append("alias_service")
            .into();
        let destination = Route::new().append(destination).into();
        Self {
            route,
            destination,
            callback_address,
            phantom_t: Default::default(),
        }
    }

    /// Create and start new RemoteMailbox with given Ockam Hub address
    /// and Address of destionation Worker that should receive forwarded messages
    pub async fn create<A: Into<Address>>(
        ctx: &mut Context,
        hub_addr: SocketAddr,
        destination: A,
    ) -> Result<RemoteMailboxInfo> {
        let remote_mailbox = Self::new(hub_addr, destination.into(), ctx.primary_address());

        let worker_address: Address = random();
        ctx.start_worker(worker_address, remote_mailbox).await?;

        let resp = ctx.receive::<RemoteMailboxInfo>().await?.take().take();

        Ok(resp)
    }
}

#[async_worker]
impl<T: Message> Worker for RemoteMailbox<T> {
    type Context = Context;
    type Message = T;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> crate::Result<()> {
        info!("RemoteMailbox registering...");
        ctx.send(self.route.clone(), "register".to_string()).await?;
        let resp = ctx.receive::<String>().await?.take();
        let route = resp.reply();
        let resp = resp.take();
        match resp.as_str() {
            "register" => self.route = route.clone(),
            _ => return Err(OckamError::InvalidHubResponse.into()),
        }
        info!("RemoteMailbox route: {}", route);
        let address;
        if let Some(a) = route.clone().recipient().to_string().strip_prefix("0#") {
            address = a.to_string();
        } else {
            return Err(OckamError::InvalidHubResponse.into());
        }

        ctx.send(
            self.callback_address.clone(),
            RemoteMailboxInfo {
                forwarding_route: route,
                remote_address: address,
                worker_address: ctx.primary_address(),
            },
        )
        .await?;

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<T>) -> Result<()> {
        let return_route = msg.reply();
        let payload = msg.take().encode()?;
        info!("RemoteMailbox received message");

        let msg = TransportMessage {
            version: 1,
            onward_route: self.destination.clone(),
            return_route,
            payload,
        };

        ctx.forward(msg).await?;

        Ok(())
    }
}
