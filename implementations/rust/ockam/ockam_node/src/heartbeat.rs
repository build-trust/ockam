use crate::Context;
use core::time::Duration;
use futures::future::{AbortHandle, Abortable};
use ockam_core::{Address, Message, Result};

/// Allow to send message to destination address periodically after some delay
/// Only one scheduled heartbeat allowed at a time
/// Dropping this handle cancels scheduled heartbeat
pub struct Heartbeat<M: Message + Clone> {
    ctx: Context,
    destination_addr: Address,
    msg: M,
    abort_handle: Option<AbortHandle>,
}

impl<M: Message + Clone> Drop for Heartbeat<M> {
    fn drop(&mut self) {
        self.cancel()
    }
}

impl<M: Message + Clone> Heartbeat<M> {
    /// Create a heartbeat
    pub async fn create(
        ctx: &Context,
        destination_addr: impl Into<Address>,
        msg: M,
    ) -> Result<Self> {
        let child_ctx = ctx.new_context(Address::random(0)).await?;

        let heartbeat = Self {
            ctx: child_ctx,
            destination_addr: destination_addr.into(),
            abort_handle: None,
            msg,
        };

        Ok(heartbeat)
    }
}

impl<M: Message + Clone> Heartbeat<M> {
    /// Cancel heartbeat
    pub fn cancel(&mut self) {
        if let Some(handle) = self.abort_handle.take() {
            handle.abort()
        }
    }

    /// Schedule heartbeat. Cancels already scheduled heartbeat if there is such heartbeat
    pub async fn schedule(&mut self, duration: Duration) -> Result<()> {
        self.cancel();

        let child_ctx = self.ctx.new_context(Address::random(0)).await?;
        let destination_addr = self.destination_addr.clone();
        let msg = self.msg.clone();

        let (handle, reg) = AbortHandle::new_pair();
        let future = Abortable::new(
            async move {
                child_ctx.sleep(duration).await;

                let res = child_ctx.send(destination_addr.clone(), msg).await;

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
    use crate::{start_node, Context, Heartbeat};
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
    #[test]
    fn scheduled_3_times__counting_worker__messages_count_matches() -> Result<()> {
        let (mut ctx, mut executor) = start_node();
        executor
            .execute(async move {
                let msgs_count = Arc::new(AtomicI8::new(0));
                let mut heartbeat = Heartbeat::create(&ctx, "counting_worker", "Hello".to_string())
                    .await
                    .unwrap();

                let worker = CountingWorker {
                    msgs_count: msgs_count.clone(),
                };

                ctx.start_worker("counting_worker", worker).await.unwrap();

                heartbeat
                    .schedule(Duration::from_millis(100))
                    .await
                    .unwrap();
                sleep(Duration::from_millis(150)).await;
                heartbeat
                    .schedule(Duration::from_millis(100))
                    .await
                    .unwrap();
                sleep(Duration::from_millis(150)).await;
                heartbeat
                    .schedule(Duration::from_millis(100))
                    .await
                    .unwrap();
                sleep(Duration::from_millis(150)).await;

                assert_eq!(3, msgs_count.load(Ordering::Relaxed));

                ctx.stop().await.unwrap();
            })
            .unwrap();

        Ok(())
    }

    #[allow(non_snake_case)]
    #[test]
    fn rescheduling__counting_worker__aborts_existing() -> Result<()> {
        let (mut ctx, mut executor) = start_node();
        executor
            .execute(async move {
                let msgs_count = Arc::new(AtomicI8::new(0));
                let mut heartbeat = Heartbeat::create(&ctx, "counting_worker", "Hello".to_string())
                    .await
                    .unwrap();

                let worker = CountingWorker {
                    msgs_count: msgs_count.clone(),
                };

                ctx.start_worker("counting_worker", worker).await.unwrap();

                heartbeat
                    .schedule(Duration::from_millis(100))
                    .await
                    .unwrap();
                heartbeat
                    .schedule(Duration::from_millis(100))
                    .await
                    .unwrap();
                heartbeat
                    .schedule(Duration::from_millis(100))
                    .await
                    .unwrap();
                sleep(Duration::from_millis(150)).await;

                assert_eq!(1, msgs_count.load(Ordering::Relaxed));

                ctx.stop().await.unwrap();
            })
            .unwrap();

        Ok(())
    }

    #[allow(non_snake_case)]
    #[test]
    fn cancel__counting_worker__aborts_existing() -> Result<()> {
        let (mut ctx, mut executor) = start_node();
        executor
            .execute(async move {
                let msgs_count = Arc::new(AtomicI8::new(0));
                let mut heartbeat = Heartbeat::create(&ctx, "counting_worker", "Hello".to_string())
                    .await
                    .unwrap();

                let worker = CountingWorker {
                    msgs_count: msgs_count.clone(),
                };

                ctx.start_worker("counting_worker", worker).await.unwrap();

                heartbeat
                    .schedule(Duration::from_millis(100))
                    .await
                    .unwrap();
                sleep(Duration::from_millis(150)).await;
                heartbeat
                    .schedule(Duration::from_millis(200))
                    .await
                    .unwrap();
                sleep(Duration::from_millis(100)).await;
                heartbeat.cancel();
                sleep(Duration::from_millis(300)).await;

                assert_eq!(1, msgs_count.load(Ordering::Relaxed));

                ctx.stop().await.unwrap();
            })
            .unwrap();

        Ok(())
    }

    #[allow(non_snake_case)]
    #[test]
    fn drop__counting_worker__aborts_existing() -> Result<()> {
        let (mut ctx, mut executor) = start_node();
        executor
            .execute(async move {
                let msgs_count = Arc::new(AtomicI8::new(0));
                let mut heartbeat = Heartbeat::create(&ctx, "counting_worker", "Hello".to_string())
                    .await
                    .unwrap();

                let worker = CountingWorker {
                    msgs_count: msgs_count.clone(),
                };

                ctx.start_worker("counting_worker", worker).await.unwrap();

                heartbeat
                    .schedule(Duration::from_millis(100))
                    .await
                    .unwrap();
                sleep(Duration::from_millis(150)).await;
                heartbeat
                    .schedule(Duration::from_millis(200))
                    .await
                    .unwrap();
                sleep(Duration::from_millis(100)).await;
                drop(heartbeat);
                sleep(Duration::from_millis(300)).await;

                assert_eq!(1, msgs_count.load(Ordering::Relaxed));

                ctx.stop().await.unwrap();
            })
            .unwrap();

        Ok(())
    }
}
