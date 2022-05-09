use crate::compat::tokio::task::JoinHandle;
use crate::Context;
use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::{Address, Message, Result};

/// Allow to send message to destination address periodically after some delay
/// Only one scheduled heartbeat allowed at a time
/// Dropping this handle cancels scheduled heartbeat
pub struct DelayedEvent<M> {
    ctx: Arc<Context>,
    destination_addr: Address,
    msg: M,
    handle: Option<JoinHandle<()>>,
}

impl<M> Drop for DelayedEvent<M> {
    fn drop(&mut self) {
        self.cancel()
    }
}

impl<M> DelayedEvent<M> {
    /// Create a heartbeat
    pub async fn create(
        ctx: &Context,
        destination_addr: impl Into<Address>,
        msg: M,
    ) -> Result<Self> {
        let child_ctx = ctx.new_context(Address::random_local()).await?;

        let heartbeat = Self {
            ctx: Arc::new(child_ctx),
            destination_addr: destination_addr.into(),
            handle: None,
            msg,
        };

        Ok(heartbeat)
    }

    /// Cancel heartbeat
    pub fn cancel(&mut self) {
        if let Some(h) = self.handle.take() {
            h.abort()
        }
    }
}

impl<M: Message + Clone> DelayedEvent<M> {
    /// Schedule heartbeat. Cancels already scheduled heartbeat if there is such heartbeat
    pub async fn schedule(&mut self, duration: Duration) -> Result<()> {
        self.cancel();

        let ctx = self.ctx.clone();
        let addr = self.destination_addr.clone();
        let msg = self.msg.clone();

        let future = async move {
            ctx.sleep(duration).await;
            if ctx.send(addr.clone(), msg).await.is_err() {
                warn!("Error sending heartbeat message to {}", addr);
            } else {
                debug!("Sent heartbeat message to {}", addr);
            }
        };

        debug_assert!(self.handle.is_none());
        self.handle = Some(self.ctx.runtime().spawn(future));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{start_node, Context, DelayedEvent};
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
                let mut heartbeat =
                    DelayedEvent::create(&ctx, "counting_worker", "Hello".to_string())
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
                let mut heartbeat =
                    DelayedEvent::create(&ctx, "counting_worker", "Hello".to_string())
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
                let mut heartbeat =
                    DelayedEvent::create(&ctx, "counting_worker", "Hello".to_string())
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
                let mut heartbeat =
                    DelayedEvent::create(&ctx, "counting_worker", "Hello".to_string())
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
