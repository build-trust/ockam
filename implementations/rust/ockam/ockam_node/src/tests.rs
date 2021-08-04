#[cfg(test)]
mod test {
    use crate::{start_node, Context};
    use async_trait::async_trait;
    use ockam_core::{route, Routed, Worker};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread::sleep;
    use std::time::Duration;

    #[allow(non_snake_case)]
    #[test]
    fn start_and_shutdown_node__many_iterations__should_not_fail() {
        for _ in 0..100 {
            let (mut ctx, mut executor) = start_node();
            executor
                .execute(async move {
                    let mut child_ctx = ctx.new_context("child").await?;
                    ctx.send(ockam_core::route!["child"], "Hello".to_string())
                        .await?;

                    let m = child_ctx.receive::<String>().await?.take().body();

                    assert_eq!(m, "Hello");

                    ctx.stop().await
                })
                .unwrap();
        }
    }

    struct SimpleWorker {
        initialize_was_called: Arc<AtomicBool>,
        shutdown_was_called: Arc<AtomicBool>,
    }

    #[async_trait]
    impl Worker for SimpleWorker {
        type Message = String;
        type Context = Context;

        async fn initialize(&mut self, _context: &mut Self::Context) -> ockam_core::Result<()> {
            self.initialize_was_called.store(true, Ordering::Relaxed);
            assert!(self.initialize_was_called.load(Ordering::Relaxed));

            Ok(())
        }

        async fn shutdown(&mut self, _context: &mut Self::Context) -> ockam_core::Result<()> {
            self.shutdown_was_called.store(true, Ordering::Relaxed);
            assert!(self.initialize_was_called.load(Ordering::Relaxed));
            assert!(self.shutdown_was_called.load(Ordering::Relaxed));

            Ok(())
        }

        async fn handle_message(
            &mut self,
            ctx: &mut Self::Context,
            msg: Routed<Self::Message>,
        ) -> ockam_core::Result<()> {
            ctx.send(msg.return_route(), msg.body()).await
        }
    }

    #[allow(non_snake_case)]
    #[test]
    fn simple_worker__run_node_lifecycle__worker_lifecycle_should_be_full() {
        let initialize_was_called = Arc::new(AtomicBool::new(false));
        let shutdown_was_called = Arc::new(AtomicBool::new(false));

        let initialize_was_called_clone = initialize_was_called.clone();
        let shutdown_was_called_clone = shutdown_was_called.clone();

        let (mut ctx, mut executor) = start_node();
        executor
            .execute(async move {
                let worker = SimpleWorker {
                    initialize_was_called: initialize_was_called_clone,
                    shutdown_was_called: shutdown_was_called_clone,
                };

                ctx.start_worker("simple_worker", worker).await.unwrap();

                ctx.send(route!["simple_worker"], "Hello".to_string())
                    .await
                    .unwrap();

                let msg = ctx.receive::<String>().await.unwrap().take().body();
                assert_eq!(msg, "Hello");

                ctx.stop().await.unwrap();
            })
            .unwrap();

        // Wait till tokio Runtime is shut down
        sleep(Duration::new(1, 0));

        assert!(initialize_was_called.load(Ordering::Relaxed));
        assert!(shutdown_was_called.load(Ordering::Relaxed));
    }
}
