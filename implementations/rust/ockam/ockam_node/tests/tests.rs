use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
};
use ockam_core::{async_trait, Address, AllowAll, Any, Decodable, DenyAll, Message, LOCAL};
use ockam_core::{route, Processor, Result, Routed, Worker};
use ockam_node::compat::futures::FutureExt;
use ockam_node::{Context, MessageReceiveOptions, NodeBuilder};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI8, AtomicU32};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::sleep;
use tracing::info;

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn receive_timeout__1_sec__should_return_from_call(ctx: &mut Context) -> Result<()> {
    let mut child_ctx = ctx.new_detached("random", AllowAll, AllowAll).await?;

    let time = SystemTime::now();
    let start = time.duration_since(UNIX_EPOCH).unwrap();
    let res = child_ctx
        .receive_extended::<String>(MessageReceiveOptions::new().with_timeout_secs(1))
        .await;
    let end = time.duration_since(UNIX_EPOCH).unwrap();
    assert!(res.is_err(), "Should not receive the message");
    let diff = end - start;
    assert!(
        diff < Duration::from_secs(2),
        "1 sec timeout definitely should not take longer than 2 secs"
    );
    ctx.stop().await
}

#[allow(non_snake_case)]
#[test]
fn start_and_shutdown_node__many_iterations__should_not_fail() {
    for _ in 0..100 {
        let (ctx, mut executor) = NodeBuilder::new().build();
        executor
            .execute(async move {
                let res = std::panic::AssertUnwindSafe(async {
                    let child_ctx1 = ctx.new_detached("child1", AllowAll, AllowAll).await?;
                    let mut child_ctx2 = ctx.new_detached("child2", AllowAll, AllowAll).await?;
                    child_ctx1
                        .send(route!["child2"], "Hello".to_string())
                        .await?;

                    let m = child_ctx2.receive::<String>().await?.body();

                    assert_eq!(m, "Hello");
                    Result::<()>::Ok(())
                })
                .catch_unwind()
                .await;

                ctx.stop().await?;

                res.unwrap()
            })
            .unwrap()
            .unwrap()
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
#[ockam_macros::test]
async fn simple_worker__run_node_lifecycle__worker_lifecycle_should_be_full(
    ctx: &mut Context,
) -> Result<()> {
    let initialize_was_called = Arc::new(AtomicBool::new(false));
    let shutdown_was_called = Arc::new(AtomicBool::new(false));

    let initialize_was_called_clone = initialize_was_called.clone();
    let shutdown_was_called_clone = shutdown_was_called.clone();

    let worker = SimpleWorker {
        initialize_was_called: initialize_was_called_clone,
        shutdown_was_called: shutdown_was_called_clone,
    };

    ctx.start_worker("simple_worker", worker).await?;

    let msg: String = ctx
        .send_and_receive(route!["simple_worker"], "Hello".to_string())
        .await?;
    assert_eq!(msg, "Hello");

    ctx.stop().await?;
    // Wait till tokio Runtime is shut down
    //    std::thread::sleep(Duration::new(1, 0));
    sleep(Duration::new(1, 0)).await;

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(shutdown_was_called.load(Ordering::Relaxed));
    Ok(())
}

struct DummyProcessor;

#[async_trait]
impl Processor for DummyProcessor {
    type Context = Context;

    async fn process(&mut self, _ctx: &mut Context) -> Result<bool> {
        Ok(true)
    }
}

#[ockam_macros::test]
async fn starting_processor_with_dup_address_should_fail(ctx: &mut Context) -> Result<()> {
    ctx.start_processor("dummy_processor", DummyProcessor)
        .await?;
    assert!(ctx
        .start_processor("dummy_processor", DummyProcessor)
        .await
        .is_err());
    ctx.stop().await
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
#[ockam_macros::test]
async fn counting_processor__run_node_lifecycle__processor_lifecycle_should_be_full(
    ctx: &mut Context,
) -> Result<()> {
    let initialize_was_called = Arc::new(AtomicBool::new(false));
    let shutdown_was_called = Arc::new(AtomicBool::new(false));
    let run_called_count = Arc::new(AtomicI8::new(0));

    let initialize_was_called_clone = initialize_was_called.clone();
    let shutdown_was_called_clone = shutdown_was_called.clone();
    let run_called_count_clone = run_called_count.clone();

    let processor = CountingProcessor {
        initialize_was_called: initialize_was_called_clone,
        shutdown_was_called: shutdown_was_called_clone,
        run_called_count: run_called_count_clone,
    };

    ctx.start_processor("counting_processor", processor).await?;
    sleep(Duration::new(1, 0)).await;

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(shutdown_was_called.load(Ordering::Relaxed));
    assert_eq!(5, run_called_count.load(Ordering::Relaxed));

    ctx.stop().await
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
#[ockam_macros::test]
async fn waiting_processor__shutdown__should_be_interrupted(ctx: &mut Context) -> Result<()> {
    let initialize_was_called = Arc::new(AtomicBool::new(false));
    let shutdown_was_called = Arc::new(AtomicBool::new(false));

    let initialize_was_called_clone = initialize_was_called.clone();
    let shutdown_was_called_clone = shutdown_was_called.clone();

    let processor = WaitingProcessor {
        initialize_was_called: initialize_was_called_clone,
        shutdown_was_called: shutdown_was_called_clone,
    };

    ctx.start_processor("waiting_processor", processor).await?;
    sleep(Duration::new(1, 0)).await;

    ctx.stop_processor("waiting_processor").await?;
    // Wait till tokio Runtime is shut down
    std::thread::sleep(Duration::new(1, 0));

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(shutdown_was_called.load(Ordering::Relaxed));
    ctx.stop().await
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
        let msg = ctx.receive::<String>().await.unwrap();
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
#[ockam_macros::test]
async fn waiting_processor__messaging__should_work(ctx: &mut Context) -> Result<()> {
    let initialize_was_called = Arc::new(AtomicBool::new(false));
    let shutdown_was_called = Arc::new(AtomicBool::new(false));

    let initialize_was_called_clone = initialize_was_called.clone();
    let shutdown_was_called_clone = shutdown_was_called.clone();

    let processor = MessagingProcessor {
        initialize_was_called: initialize_was_called_clone,
        shutdown_was_called: shutdown_was_called_clone,
    };

    ctx.start_processor_with_access_control("messaging_processor", processor, AllowAll, AllowAll)
        .await?;
    sleep(Duration::new(1, 0)).await;

    let msg: String = ctx
        .send_and_receive(route!["messaging_processor"], "Keep working".to_string())
        .await?;
    assert_eq!("OK", msg);

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(!shutdown_was_called.load(Ordering::Relaxed));

    let msg: String = ctx
        .send_and_receive(route!["messaging_processor"], "Stop working".to_string())
        .await?;
    assert_eq!("I go home", msg);

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(shutdown_was_called.load(Ordering::Relaxed));

    ctx.stop().await
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
#[ockam_macros::test]
async fn abort_blocked_shutdown(ctx: &mut Context) -> Result<()> {
    // Create an executor
    ctx.start_worker_with_access_control("bad", BadWorker, DenyAll, DenyAll)
        .await?;

    ockam_node::tokio::time::timeout(Duration::from_secs(2), ctx.stop())
        .await
        .unwrap()
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

#[ockam_macros::test]
async fn wait_for_worker(ctx: &mut Context) -> Result<()> {
    let t1 = tokio::time::Instant::now();
    ctx.start_worker_with_access_control("slow", WaitForWorker, DenyAll, DenyAll)
        .await
        .unwrap();

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
/// See https://github.com/build-trust/ockam/issues/2283
/// See https://github.com/build-trust/ockam/issues/2280
#[ockam_macros::test]
async fn worker_calls_stopworker_from_handlemessage(ctx: &mut Context) -> Result<()> {
    let counter_a = Arc::new(AtomicU32::new(0));
    let counter_b = Arc::new(AtomicU32::new(0));
    let counter_a_clone = counter_a.clone();
    let counter_b_clone = counter_b.clone();

    let child_ctx = ctx.new_detached("child", AllowAll, AllowAll).await?;

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
            ctx.start_worker(addr.clone(), worker).await.unwrap();
            addrs.push(addr);
        }

        let mut join_handles = Vec::new();
        for addr in addrs {
            join_handles.push(child_ctx.send(route![addr], String::from("Testing. 1. 2. 3.")));
        }

        for h in join_handles {
            h.await.unwrap();
        }
    }
    // Wait till tokio Runtime is shut down
    std::thread::sleep(Duration::new(1, 0));

    // Assert all handle_message() entry and exit counts match
    assert_eq!(
        counter_a.load(Ordering::Relaxed),
        counter_b.load(Ordering::Relaxed)
    );
    ctx.stop().await
}

struct SendReceiveWorker;

#[async_trait]
impl Worker for SendReceiveWorker {
    type Context = Context;
    type Message = Any;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let return_route = msg.return_route();
        let msg = SendReceiveRequest::decode(msg.payload())?;

        match msg {
            SendReceiveRequest::Connect() => {
                ctx.send(return_route, SendReceiveResponse::Connect(Ok(())))
                    .await?;
            }
        }

        ctx.stop().await
    }
}

#[derive(Serialize, Deserialize, Debug, Message)]
enum SendReceiveRequest {
    Connect(),
}

#[derive(Serialize, Deserialize, Debug, Message)]
enum SendReceiveResponse {
    Connect(Result<()>),
}

/// Test the new method Context::send_and_receive().
/// See https://github.com/build-trust/ockam/issues/2628.
#[ockam_macros::test]
async fn use_context_send_and_receive(ctx: &mut Context) -> Result<()> {
    ctx.start_worker("SendReceiveWorker", SendReceiveWorker)
        .await?;

    let msg_tx = SendReceiveRequest::Connect();
    let msg_rx = ctx.send_and_receive("SendReceiveWorker", msg_tx).await?;

    if let SendReceiveResponse::Connect(Err(e)) = msg_rx {
        panic!("test failure: {}", e)
    }
    ctx.stop().await
}

struct DummyWorker;

#[async_trait]
impl Worker for DummyWorker {
    type Message = String;
    type Context = Context;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
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

#[ockam_macros::test]
async fn starting_worker_with_dup_address_should_fail(ctx: &mut Context) -> Result<()> {
    ctx.start_worker_with_access_control("dummy_worker", DummyWorker, DenyAll, DenyAll)
        .await?;
    assert!(ctx
        .start_worker_with_access_control("dummy_worker", DummyWorker, DenyAll, DenyAll)
        .await
        .is_err());
    ctx.stop().await
}
