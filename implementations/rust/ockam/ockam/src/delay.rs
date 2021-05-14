use crate::{spawn, Address, Context, Message, Result, Route};
use rand::random;
use std::time::Duration;

/// Send a delayed event to a worker
pub struct DelayedEvent<M: Message> {
    route: Route,
    ctx: Context,
    d: Duration,
    msg: M,
}

impl<M: Message> DelayedEvent<M> {
    /// Create a new 100ms delayed message event
    pub async fn new(ctx: &Context, route: Route, msg: M) -> Result<Self> {
        Ok(Self {
            route,
            ctx: ctx.new_context(random::<Address>()).await?,
            d: Duration::from_millis(100),
            msg,
        })
    }

    /// Adjust the delay time in seconds
    pub fn seconds(self, s: u64) -> Self {
        Self {
            d: Duration::from_secs(s),
            ..self
        }
    }

    /// Adjust the delay time in milliseconds
    pub fn millis(self, s: u64) -> Self {
        Self {
            d: Duration::from_millis(s),
            ..self
        }
    }

    /// Run this delayed event
    pub fn spawn(self) {
        let Self { route, ctx, d, msg } = self;
        spawn(async move {
            ctx.sleep(d).await;
            if let Err(e) = ctx.send(route, msg).await {
                error!("Failed to send delayed message: {}", e);
            }
        })
    }
}
