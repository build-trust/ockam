use crate::ChannelError;
use ockam::{
    async_worker, Address, Context, Message, Result, Route, Routed, TransportMessage, Worker,
};
use std::marker::PhantomData;
use std::net::SocketAddr;
use tracing::info;

/// This Worker is responsible for registering on Ockam Hub and forwarding message with type T to
/// local Worker
pub struct HubProxy<T: Message> {
    route: Route,
    proxy_dest: Route,
    phantom_t: PhantomData<T>,
}

impl<T: Message> HubProxy<T> {
    /// Create  new HubProxy with given Ockam Hub address and Address of Worker that
    /// should receive forwarded messages
    pub fn new(hub_addr: SocketAddr, proxy_dest: Address) -> Self {
        let route = Route::new()
            .append(format!("1#{}", hub_addr))
            .append("alias_service")
            .into();
        let proxy_dest = Route::new().append(proxy_dest).into();
        Self {
            route,
            proxy_dest,
            phantom_t: Default::default(),
        }
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
            "register" => self.route = route,
            _ => return Err(ChannelError::InvalidHubResponse.into()),
        }
        info!("Hub route: {}", self.route);

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
