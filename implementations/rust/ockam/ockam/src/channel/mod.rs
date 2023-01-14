//! Ockam general bi-directional channel

mod listener;
mod worker;

use self::{listener::ChannelListener, worker::ChannelWorker};
use crate::{
    pipe::{BehaviorHook, PipeBehavior},
    Context,
};
use ockam_core::{Address, DenyAll, Result, Route, RouteBuilder};

const CLUSTER_NAME: &str = "ockam.channel";

/// Generalised ockam channel API
pub struct ChannelBuilder {
    ctx: Context,
    tx_hooks: PipeBehavior,
    rx_hooks: PipeBehavior,
}

impl ChannelBuilder {
    /// Create a new Ockam channel context
    ///
    /// ```rust
    /// # use ockam::{channel::ChannelBuilder, Context};
    /// # use ockam_core::Result;
    /// # async fn test_api(ctx: &mut Context) -> Result<()> {
    /// let builder = ChannelBuilder::new(ctx).await?;
    ///
    /// // Create a new channel listener
    /// builder.create_channel_listener("my-channel-listener").await?;
    ///
    /// // Connect a channel to the listener
    /// let ch = builder.connect("my-channel-listener").await?;
    /// ctx.send(ch.tx().append("app"), String::from("Hello via channel!")).await?;
    ///
    /// // Wait for the reply message
    /// let msg = ctx.receive::<String>().await?;
    /// println!("Received message '{}'", msg);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(ctx: &Context) -> Result<Self> {
        debug!("Creating new ChannelBuilder context...");
        ctx.new_detached(Address::random_local(), DenyAll, DenyAll)
            .await
            .map(|ctx| Self {
                ctx,
                tx_hooks: PipeBehavior::empty(),
                rx_hooks: PipeBehavior::empty(),
            })
    }

    /// Attach a new behavior to be used by the underlying pipe sender
    pub fn attach_tx_behavior<B: BehaviorHook + Clone + Send + Sync + 'static>(
        mut self,
        bev: B,
    ) -> Self {
        self.tx_hooks.insert(bev);
        self
    }

    /// Attach a new behavior to be used by the underlying pipe receiver
    pub fn attach_rx_behavior<B: BehaviorHook + Clone + Send + Sync + 'static>(
        mut self,
        bev: B,
    ) -> Self {
        self.rx_hooks.insert(bev);
        self
    }

    /// Connect to a channel listener
    pub async fn connect<R: Into<Route>>(&self, listener: R) -> Result<ChannelHandle> {
        let tx = Address::random_local();
        ChannelWorker::stage1(
            &self.ctx,
            tx.clone(),
            listener.into(),
            PipeBehavior::empty(),
            PipeBehavior::empty(),
        )
        .await?;
        Ok(ChannelHandle { tx })
    }

    /// Create a new channel listener
    pub async fn create_channel_listener<A: Into<Address>>(&self, addr: A) -> Result<()> {
        ChannelListener::create(
            &self.ctx,
            addr.into(),
            self.tx_hooks.clone(),
            self.rx_hooks.clone(),
        )
        .await
    }
}

/// A handle which may be used to send data through a channel.
///
/// This is implemented as a type-safe wrapper around an address.
pub struct ChannelHandle {
    tx: Address,
}

impl ChannelHandle {
    /// Returns a route-builder for sending data through this channel.
    pub fn tx(&self) -> RouteBuilder {
        RouteBuilder::new().prepend_route(self.tx.clone().into())
    }

    /// Returns the underlying address
    pub fn address(&self) -> &Address {
        &self.tx
    }
}

impl From<ChannelHandle> for Address {
    fn from(ch: ChannelHandle) -> Address {
        ch.tx
    }
}
