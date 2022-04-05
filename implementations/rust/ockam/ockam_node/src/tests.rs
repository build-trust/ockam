use crate::{start_node, Context};
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
};
use ockam_core::{async_trait, Address, LOCAL};
use ockam_core::{route, Processor, Result, Routed, Worker};
use std::sync::atomic::{AtomicI8, AtomicU32};
use tokio::time::sleep;

#[allow(non_snake_case)]
#[test]
fn start_and_shutdown_node__many_iterations__should_not_fail() {
    for _ in 0..100 {
        let (mut ctx, mut executor) = start_node();
        executor
            .execute(async move {
                let mut child_ctx = ctx.new_context("child").await?;
                ctx.send(route!["child"], "Hello".to_string()).await?;

                let m = child_ctx.receive::<String>().await?.take().body();

                assert_eq!(m, "Hello");

                ctx.stop().await
            })
            .unwrap()
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

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.initialize_was_called.store(true, Ordering::Relaxed);
        assert!(self.initialize_was_called.load(Ordering::Relaxed));

        Ok(())
    }

    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.shutdown_was_called.store(true, Ordering::Relaxed);
        assert!(self.initialize_was_called.load(Ordering::Relaxed));
        assert!(self.shutdown_was_called.load(Ordering::Relaxed));

        Ok(())
    }

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
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
    std::thread::sleep(Duration::new(1, 0));

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(shutdown_was_called.load(Ordering::Relaxed));
}

struct CountingProcessor {
    initialize_was_called: Arc<AtomicBool>,
    shutdown_was_called: Arc<AtomicBool>,
    run_called_count: Arc<AtomicI8>,
}

#[async_trait]
impl Processor for CountingProcessor {
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.initialize_was_called.store(true, Ordering::Relaxed);
        assert!(self.initialize_was_called.load(Ordering::Relaxed));

        Ok(())
    }

    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.shutdown_was_called.store(true, Ordering::Relaxed);
        assert!(self.initialize_was_called.load(Ordering::Relaxed));
        assert!(self.shutdown_was_called.load(Ordering::Relaxed));

        Ok(())
    }

    async fn process(&mut self, _ctx: &mut Self::Context) -> Result<bool> {
        let val = self.run_called_count.fetch_add(1, Ordering::Relaxed);

        Ok(val < 4)
    }
}

#[allow(non_snake_case)]
#[test]
fn counting_processor__run_node_lifecycle__processor_lifecycle_should_be_full() {
    let initialize_was_called = Arc::new(AtomicBool::new(false));
    let shutdown_was_called = Arc::new(AtomicBool::new(false));
    let run_called_count = Arc::new(AtomicI8::new(0));

    let initialize_was_called_clone = initialize_was_called.clone();
    let shutdown_was_called_clone = shutdown_was_called.clone();
    let run_called_count_clone = run_called_count.clone();

    let (mut ctx, mut executor) = start_node();
    executor
        .execute(async move {
            let processor = CountingProcessor {
                initialize_was_called: initialize_was_called_clone,
                shutdown_was_called: shutdown_was_called_clone,
                run_called_count: run_called_count_clone,
            };

            ctx.start_processor("counting_processor", processor)
                .await
                .unwrap();
            sleep(Duration::new(1, 0)).await;

            assert!(initialize_was_called.load(Ordering::Relaxed));
            assert!(shutdown_was_called.load(Ordering::Relaxed));
            assert_eq!(5, run_called_count.load(Ordering::Relaxed));

            ctx.stop().await.unwrap();
        })
        .unwrap();
}

struct WaitingProcessor {
    initialize_was_called: Arc<AtomicBool>,
    shutdown_was_called: Arc<AtomicBool>,
}

#[async_trait]
impl Processor for WaitingProcessor {
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.initialize_was_called.store(true, Ordering::Relaxed);
        assert!(self.initialize_was_called.load(Ordering::Relaxed));

        Ok(())
    }

    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.shutdown_was_called.store(true, Ordering::Relaxed);
        assert!(self.initialize_was_called.load(Ordering::Relaxed));
        assert!(self.shutdown_was_called.load(Ordering::Relaxed));

        Ok(())
    }

    async fn process(&mut self, _ctx: &mut Self::Context) -> Result<bool> {
        sleep(Duration::new(1, 0)).await;

        Ok(true)
    }
}

#[allow(non_snake_case)]
#[test]
fn waiting_processor__shutdown__should_be_interrupted() {
    let initialize_was_called = Arc::new(AtomicBool::new(false));
    let shutdown_was_called = Arc::new(AtomicBool::new(false));

    let initialize_was_called_clone = initialize_was_called.clone();
    let shutdown_was_called_clone = shutdown_was_called.clone();

    let (mut ctx, mut executor) = start_node();
    executor
        .execute(async move {
            let processor = WaitingProcessor {
                initialize_was_called: initialize_was_called_clone,
                shutdown_was_called: shutdown_was_called_clone,
            };

            ctx.start_processor("waiting_processor", processor)
                .await
                .unwrap();
            sleep(Duration::new(1, 0)).await;

            ctx.stop_processor("waiting_processor").await.unwrap();

            ctx.stop().await.unwrap();
        })
        .unwrap();

    // Wait till tokio Runtime is shut down
    std::thread::sleep(Duration::new(1, 0));

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(shutdown_was_called.load(Ordering::Relaxed));
}

struct MessagingProcessor {
    initialize_was_called: Arc<AtomicBool>,
    shutdown_was_called: Arc<AtomicBool>,
}

#[async_trait]
impl Processor for MessagingProcessor {
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.initialize_was_called.store(true, Ordering::Relaxed);
        assert!(self.initialize_was_called.load(Ordering::Relaxed));

        Ok(())
    }

    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.shutdown_was_called.store(true, Ordering::Relaxed);
        assert!(self.initialize_was_called.load(Ordering::Relaxed));
        assert!(self.shutdown_was_called.load(Ordering::Relaxed));

        Ok(())
    }

    async fn process(&mut self, ctx: &mut Self::Context) -> Result<bool> {
        let msg = ctx.receive::<String>().await.unwrap().take();
        let route = msg.return_route();
        let body = msg.body();

        match body.as_str() {
            "Keep working" => {
                ctx.send(route, "OK".to_string()).await?;
                Ok(true)
            }
            "Stop working" => {
                ctx.send(route, "I go home".to_string()).await?;
                Ok(false)
            }
            _ => panic!(),
        }
    }
}

#[allow(non_snake_case)]
#[test]
fn waiting_processor__messaging__should_work() {
    let initialize_was_called = Arc::new(AtomicBool::new(false));
    let shutdown_was_called = Arc::new(AtomicBool::new(false));

    let initialize_was_called_clone = initialize_was_called.clone();
    let shutdown_was_called_clone = shutdown_was_called.clone();

    let (mut ctx, mut executor) = start_node();
    executor
        .execute(async move {
            let processor = MessagingProcessor {
                initialize_was_called: initialize_was_called_clone,
                shutdown_was_called: shutdown_was_called_clone,
            };

            ctx.start_processor("messaging_processor", processor)
                .await
                .unwrap();
            sleep(Duration::new(1, 0)).await;

            ctx.send(route!["messaging_processor"], "Keep working".to_string())
                .await
                .unwrap();
            assert_eq!("OK", ctx.receive::<String>().await.unwrap().take().body());

            assert!(initialize_was_called.load(Ordering::Relaxed));
            assert!(!shutdown_was_called.load(Ordering::Relaxed));

            ctx.send(route!["messaging_processor"], "Stop working".to_string())
                .await
                .unwrap();
            assert_eq!(
                "I go home",
                ctx.receive::<String>().await.unwrap().take().body()
            );

            assert!(initialize_was_called.load(Ordering::Relaxed));
            assert!(shutdown_was_called.load(Ordering::Relaxed));

            ctx.stop().await.unwrap();
        })
        .unwrap();
}

struct BadWorker;

#[ockam_core::worker]
impl Worker for BadWorker {
    type Context = Context;
    type Message = ();

    /// This shutdown function takes _way_ too long to complete
    async fn shutdown(&mut self, ctx: &mut Context) -> Result<()> {
        ctx.sleep(Duration::from_secs(10)).await;
        Ok(())
    }
}

/// This test enforces that a shutdown that is blocked by a worker
/// will be aborted eventually.
#[test]
fn abort_blocked_shutdown() {
    // Create an executor
    let (mut ctx, mut executor) = start_node();
    executor
        .execute(async move {
            ctx.start_worker("bad", BadWorker).await?;

            crate::tokio::time::timeout(Duration::from_secs(2), async { ctx.stop().await })
                .await
                .unwrap()
        })
        .unwrap()
        .unwrap();
}

struct WaitForWorker;

#[ockam_core::worker]
impl Worker for WaitForWorker {
    type Context = Context;
    type Message = ();

    async fn initialize(&mut self, ctx: &mut Context) -> Result<()> {
        info!("This worker initialises a bit slow");
        ctx.sleep(Duration::from_secs(1)).await;
        info!("Worker done");
        Ok(())
    }
}

#[ockam_macros::test(crate = "crate")]
async fn wait_for_worker(ctx: &mut Context) -> Result<()> {
    let t1 = tokio::time::Instant::now();
    ctx.start_worker("slow", WaitForWorker).await.unwrap();

    info!("Waiting for worker...");
    ctx.wait_for("slow").await.unwrap();
    info!("Done waiting :)");

    let t2 = tokio::time::Instant::now();
    assert!((t2 - t1) > Duration::from_secs(1));

    if let Err(e) = ctx.stop().await {
        println!("Unclean stop: {}", e)
    }
    Ok(())
}

/// Test the, unexpected, case where a payload is received that does not
/// code its length at the start. This _may_ happen when dealing with a
/// payload sent by a non-Rust implementation.
/// See https://github.com/ockam-network/ockam/issues/2236.
#[test]
fn parse_payload_without_inner_length() {
    use crate::parser;

    // A well formed String payload of 32 smiley chars.
    let payload: [u8; 130] = [
        128, 1, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128,
        240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159,
        152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128,
        240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159,
        152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128,
        240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159,
        152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128, 240, 159, 152, 128,
        240, 159, 152, 128,
    ];
    let r = parser::message::<String>(&payload).unwrap();
    assert_eq!("😀".repeat(32), r);

    // A String payload of 32 smiley chars that is missing its inner length.
    let payload = "😀".repeat(32);
    let r = parser::message::<String>(payload.as_bytes()).unwrap();
    assert_eq!("😀".repeat(32), r);

    // A 100KiB String payload of smiley chars that is missing its inner length.
    let payload = "😀".repeat(25600);
    let r = parser::message::<String>(payload.as_bytes()).unwrap();
    assert_eq!("😀".repeat(25600), r);
}

struct StopFromHandleMessageWorker {
    counter_a: Arc<AtomicU32>,
    counter_b: Arc<AtomicU32>,
}

#[async_trait]
impl Worker for StopFromHandleMessageWorker {
    type Message = String;
    type Context = Context;
    async fn handle_message(&mut self, ctx: &mut Context, _msg: Routed<String>) -> Result<()> {
        self.counter_a.fetch_add(1, Ordering::Relaxed);
        ctx.stop_worker(ctx.address()).await?;
        self.counter_b.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

/// Test that a Worker can complete execution of its handle_message()
/// even if it calls Context::stop_worker() from within handle_message().
/// See https://github.com/ockam-network/ockam/issues/2283
/// See https://github.com/ockam-network/ockam/issues/2280
#[test]
fn worker_calls_stopworker_from_handlemessage() {
    let (mut ctx, mut executor) = start_node();

    let counter_a = Arc::new(AtomicU32::new(0));
    let counter_b = Arc::new(AtomicU32::new(0));
    let counter_a_clone = counter_a.clone();
    let counter_b_clone = counter_b.clone();

    executor
        .execute(async move {
            const RUNS: u32 = 1000;
            const WORKERS: u32 = 10;
            for _ in 0..RUNS {
                let mut addrs = Vec::new();
                for _ in 0..WORKERS {
                    let worker = StopFromHandleMessageWorker {
                        counter_a: counter_a_clone.clone(),
                        counter_b: counter_b_clone.clone(),
                    };
                    let addr = Address::random(LOCAL);
                    ctx.start_worker(&addr, worker).await.unwrap();
                    addrs.push(addr);
                }

                let mut join_handles = Vec::new();
                for addr in addrs {
                    join_handles.push(ctx.send(route![addr], String::from("Testing. 1. 2. 3.")));
                }

                for h in join_handles {
                    h.await.unwrap();
                }
            }

            ctx.stop().await.unwrap();
        })
        .unwrap();

    // Wait till tokio Runtime is shut down
    std::thread::sleep(Duration::new(1, 0));

    // Assert all handle_message() entry and exit counts match
    assert_eq!(
        counter_a.load(Ordering::Relaxed),
        counter_b.load(Ordering::Relaxed)
    );
}
