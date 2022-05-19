use crate::{Address, Context, Message, Result, Route};
use core::time::Duration;

/// Send a delayed event to a worker
pub(crate) struct DelayedEvent<M: Message> {
    route: Route,
    ctx: Context,
    d: Duration,
    msg: M,
}

impl<M: Message> DelayedEvent<M> {
    /// Create a new 100ms delayed message event
    pub(crate) async fn new(ctx: &Context, route: Route, msg: M) -> Result<Self> {
        let child_ctx = ctx.new_detached(Address::random_local()).await?;

        debug!(
            "Creating a delayed event with address '{}'",
            child_ctx.address()
        );

        Ok(Self {
            route,
            ctx: child_ctx,
            d: Duration::from_millis(100),
            msg,
        })
    }

    /// Adjust the delay time with a [`Duration`](core::time::Duration)
    pub(crate) fn with_duration(self, d: Duration) -> Self {
        Self { d, ..self }
    }

    /// Adjust the delay time in seconds
    pub(crate) fn with_seconds(self, secs: u64) -> Self {
        Self {
            d: Duration::from_secs(secs),
            ..self
        }
    }

    /// Run this delayed event
    pub(crate) fn spawn(self) {
        let Self { route, ctx, d, msg } = self;
        ockam_node::spawn(async move {
            ctx.sleep(d).await;
            if let Err(e) = ctx.send(route, msg).await {
                error!("Failed to send delayed message: {}", e);
            }
        })
    }
}
