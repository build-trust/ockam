use crate::ChannelError;
use ockam::{
    async_worker, Address, Context, Message, Result, Route, Routed, TransportMessage, Worker,
};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::net::SocketAddr;
use tracing::info;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ProxyRegistered {
    route: Route,
    address: String,
}

impl ProxyRegistered {
    pub fn route(&self) -> &Route {
        &self.route
    }
    pub fn address(&self) -> &str {
        &self.address
    }
}

impl ProxyRegistered {
    pub(crate) fn new(route: Route, address: String) -> Self {
        ProxyRegistered { route, address }
    }
}

/// This Worker is responsible for registering on Ockam Hub and forwarding message with type T to
/// local Worker
pub struct HubProxy<T: Message> {
    route: Route,
    proxy_dest: Route,
    callback_route: Route,
    phantom_t: PhantomData<T>,
}

impl<T: Message> HubProxy<T> {
    fn new(hub_addr: SocketAddr, proxy_dest: Address, callback_route: Route) -> Self {
        let route = Route::new()
            .append(format!("1#{}", hub_addr))
            .append("alias_service")
            .into();
        let proxy_dest = Route::new().append(proxy_dest).into();
        Self {
            route,
            proxy_dest,
            callback_route,
            phantom_t: Default::default(),
        }
    }

    /// Create and start new HubProxy with given Ockam Hub address and Address of Worker that
    /// should receive forwarded messages
    pub async fn register_proxy(
        ctx: &Context,
        worker_address: Address,
        hub_addr: SocketAddr,
        proxy_dest: Address,
    ) -> Result<()> {
        let proxy = Self::new(
            hub_addr,
            proxy_dest,
            Route::new().append(ctx.address()).into(),
        );

        ctx.start_worker(worker_address, proxy).await?;

        Ok(())
    }
}

#[async_worker]
impl<T: Message> Worker for HubProxy<T> {
    type Context = Context;
    type Message = T;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        info!("Hub registering...");
        ctx.send_message(self.route.clone(), "register".to_string())
            .await?;
        let resp = ctx.receive::<String>().await?.take();
        let route = resp.reply();
        let resp = resp.take();
        match resp.as_str() {
            "register" => self.route = route.clone(),
            _ => return Err(ChannelError::InvalidHubResponse.into()),
        }
        info!("Hub route: {}", route);
        let address;
        if let Some(a) = route.clone().recipient().to_string().strip_prefix("0#") {
            address = a.to_string();
        } else {
            return Err(ChannelError::InvalidHubResponse.into());
        }

        ctx.send_message(
            self.callback_route.clone(),
            ProxyRegistered::new(route.clone(), address),
        )
        .await?;

        Ok(())
    }

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<T>) -> Result<()> {
        let return_route = msg.reply();
        let payload = msg.take().encode()?;
        info!("Hub proxy received message");

        let msg = TransportMessage {
            version: 1,
            onward: self.proxy_dest.clone(),
            return_: return_route,
            payload,
        };

        ctx.forward_message(msg).await?;

        Ok(())
    }
}
