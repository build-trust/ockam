use crate::{block_future, spawn, Address, Context, Message, Result, Route};
use core::time::Duration;

/// Send a delayed event to a worker
pub struct DelayedEvent<M: Message> {
    route: Route,
    ctx: Context,
    d: Duration,
    msg: M,
}

impl<M: Message> DelayedEvent<M> {
    /// Create a new 100ms delayed message event
    pub fn new(ctx: &Context, route: Route, msg: M) -> Result<Self> {
        let ctx = block_future(&ctx.runtime(), async {
            ctx.new_context(Address::random(0)).await
        })?;

        debug!("Creating a delayed event with address '{}'", ctx.address());

        Ok(Self {
            route,
            ctx,
            d: Duration::from_millis(100),
            msg,
        })
    }

    /// Adjust the delay time with a [`Duration`](core::time::Duration)
    pub fn with_duration(self, d: Duration) -> Self {
        Self { d, ..self }
    }

    /// Adjust the delay time in milliseconds
    pub fn with_millis(self, millis: u64) -> Self {
        Self {
            d: Duration::from_millis(millis),
            ..self
        }
    }

    /// Adjust the delay time in seconds
    pub fn with_seconds(self, secs: u64) -> Self {
        Self {
            d: Duration::from_secs(secs),
            ..self
        }
    }

    /// Adjust the delay time in minutes
    pub fn with_minutes(self, mins: u64) -> Self {
        Self {
            d: Duration::from_secs(mins * 60),
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

    /// Block on this delayed event, returning the result
    pub fn block(self) -> Result<()> {
        let Self { route, ctx, d, msg } = self;
        block_future(&ctx.runtime(), async move {
            ctx.sleep(d).await;
            ctx.send(route, msg).await
        })
    }
}
