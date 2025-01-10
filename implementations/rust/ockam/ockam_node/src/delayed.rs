use crate::Context;
use core::time::Duration;
use futures::future::{AbortHandle, Abortable};
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, AllowOnwardAddress, DenyAll, Mailboxes, Message, Result};

/// Allow to send message to destination address periodically after some delay
/// Only one scheduled heartbeat allowed at a time
/// Dropping this handle cancels scheduled heartbeat
pub struct DelayedEvent<M: Message + Clone> {
    ctx: Arc<Context>,
    destination_addr: Address,
    msg: M,
    abort_handle: Option<AbortHandle>,
}

impl<M: Message + Clone> Drop for DelayedEvent<M> {
    fn drop(&mut self) {
        self.cancel()
    }
}

impl<M: Message + Clone> DelayedEvent<M> {
    /// Create a heartbeat
    pub fn create(ctx: &Context, destination_addr: impl Into<Address>, msg: M) -> Result<Self> {
        let destination_addr = destination_addr.into();
        let mailboxes = Mailboxes::primary(
            Address::random_tagged("DelayedEvent.create"),
            Arc::new(DenyAll),
            Arc::new(AllowOnwardAddress(destination_addr.clone())),
        );
        let child_ctx = ctx.new_detached_with_mailboxes(mailboxes)?;

        let heartbeat = Self {
            ctx: Arc::new(child_ctx),
            destination_addr,
            abort_handle: None,
            msg,
        };

        Ok(heartbeat)
    }

    /// Address used to send messages to destination address
    pub fn address(&self) -> &Address {
        self.ctx.primary_address()
    }
}

impl<M: Message + Clone> DelayedEvent<M> {
    /// Cancel heartbeat
    pub fn cancel(&mut self) {
        if let Some(handle) = self.abort_handle.take() {
            handle.abort()
        }
    }

    /// Schedule heartbeat. Cancels already scheduled heartbeat if there is such heartbeat
    pub fn schedule(&mut self, duration: Duration) -> Result<()> {
        self.cancel();

        let destination_addr = self.destination_addr.clone();
        let msg = self.msg.clone();

        let ctx_clone = self.ctx.clone();
        let (handle, reg) = AbortHandle::new_pair();
        let future = Abortable::new(
            async move {
                ctx_clone.sleep(duration).await;

                let res = ctx_clone.send(destination_addr.clone(), msg).await;

                if res.is_err() {
                    warn!("Error sending heartbeat message to {}", destination_addr);
                } else {
                    debug!("Sent heartbeat message to {}", destination_addr);
                }
            },
            reg,
        );

        self.abort_handle = Some(handle);
        self.ctx.runtime().spawn(future);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Context, DelayedEvent};
    use core::sync::atomic::Ordering;
    use core::time::Duration;
    use ockam_core::compat::{boxed::Box, string::ToString, sync::Arc};
    use ockam_core::{async_trait, Any};
    use ockam_core::{Result, Routed, Worker};
    use std::sync::atomic::AtomicI8;
    use tokio::time::sleep;

    struct CountingWorker {
        msgs_count: Arc<AtomicI8>,
    }

    #[async_trait]
    impl Worker for CountingWorker {
        type Context = Context;
        type Message = Any;

        async fn handle_message(
            &mut self,
            _context: &mut Self::Context,
            _msg: Routed<Self::Message>,
        ) -> Result<()> {
            let _ = self.msgs_count.fetch_add(1, Ordering::Relaxed);

            Ok(())
        }
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(crate = "crate")]
    async fn scheduled_3_times__counting_worker__messages_count_matches(
        ctx: &mut Context,
    ) -> Result<()> {
        let msgs_count = Arc::new(AtomicI8::new(0));
        let mut heartbeat = DelayedEvent::create(ctx, "counting_worker", "Hello".to_string())?;

        let worker = CountingWorker {
            msgs_count: msgs_count.clone(),
        };

        ctx.start_worker("counting_worker", worker)?;

        heartbeat.schedule(Duration::from_millis(100))?;
        sleep(Duration::from_millis(150)).await;
        heartbeat.schedule(Duration::from_millis(100))?;
        sleep(Duration::from_millis(150)).await;
        heartbeat.schedule(Duration::from_millis(100))?;
        sleep(Duration::from_millis(150)).await;

        assert_eq!(3, msgs_count.load(Ordering::Relaxed));
        Ok(())
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(crate = "crate")]
    async fn rescheduling__counting_worker__aborts_existing(ctx: &mut Context) -> Result<()> {
        let msgs_count = Arc::new(AtomicI8::new(0));
        let mut heartbeat = DelayedEvent::create(ctx, "counting_worker", "Hello".to_string())?;

        let worker = CountingWorker {
            msgs_count: msgs_count.clone(),
        };

        ctx.start_worker("counting_worker", worker)?;

        heartbeat.schedule(Duration::from_millis(100))?;
        heartbeat.schedule(Duration::from_millis(100))?;
        heartbeat.schedule(Duration::from_millis(100))?;
        sleep(Duration::from_millis(150)).await;

        assert_eq!(1, msgs_count.load(Ordering::Relaxed));
        Ok(())
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(crate = "crate")]
    async fn cancel__counting_worker__aborts_existing(ctx: &mut Context) -> Result<()> {
        let msgs_count = Arc::new(AtomicI8::new(0));
        let mut heartbeat = DelayedEvent::create(ctx, "counting_worker", "Hello".to_string())?;

        let worker = CountingWorker {
            msgs_count: msgs_count.clone(),
        };

        ctx.start_worker("counting_worker", worker)?;

        heartbeat.schedule(Duration::from_millis(100))?;
        sleep(Duration::from_millis(150)).await;
        heartbeat.schedule(Duration::from_millis(200))?;
        sleep(Duration::from_millis(100)).await;
        heartbeat.cancel();
        sleep(Duration::from_millis(300)).await;

        assert_eq!(1, msgs_count.load(Ordering::Relaxed));
        Ok(())
    }

    #[allow(non_snake_case)]
    #[ockam_macros::test(crate = "crate")]
    async fn drop__counting_worker__aborts_existing(ctx: &mut Context) -> Result<()> {
        let msgs_count = Arc::new(AtomicI8::new(0));
        let mut heartbeat = DelayedEvent::create(ctx, "counting_worker", "Hello".to_string())?;

        let worker = CountingWorker {
            msgs_count: msgs_count.clone(),
        };

        ctx.start_worker("counting_worker", worker)?;

        heartbeat.schedule(Duration::from_millis(100))?;
        sleep(Duration::from_millis(150)).await;
        heartbeat.schedule(Duration::from_millis(200))?;
        sleep(Duration::from_millis(100)).await;
        drop(heartbeat);
        sleep(Duration::from_millis(300)).await;

        assert_eq!(1, msgs_count.load(Ordering::Relaxed));
        Ok(())
    }
}
