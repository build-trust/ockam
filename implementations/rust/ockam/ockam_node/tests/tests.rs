use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;
use ockam_core::compat::{
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, Address, AllowAll, Any, Decodable, DenyAll, Message};
use ockam_core::{route, Processor, Result, Routed, Worker};
use ockam_node::compat::futures::FutureExt;
use ockam_node::{Context, MessageReceiveOptions, NodeBuilder};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicI8;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::sleep;

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn receive_timeout__1_sec__should_return_from_call(ctx: &mut Context) -> Result<()> {
    let mut child_ctx = ctx.new_detached("random", AllowAll, AllowAll)?;

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
    Ok(())
}

#[allow(non_snake_case)]
#[test]
fn start_and_shutdown_node__many_iterations__should_not_fail() {
    for _ in 0..100 {
        let (ctx, mut executor) = NodeBuilder::new().build();
        executor
            .execute(async move {
                let res = std::panic::AssertUnwindSafe(async {
                    let child_ctx1 = ctx.new_detached("child1", AllowAll, AllowAll)?;
                    let mut child_ctx2 = ctx.new_detached("child2", AllowAll, AllowAll)?;
                    child_ctx1
                        .send(route!["child2"], "Hello".to_string())
                        .await?;

                    let m = child_ctx2.receive::<String>().await?.into_body()?;

                    assert_eq!(m, "Hello");
                    Result::<()>::Ok(())
                })
                .catch_unwind()
                .await;

                ctx.shutdown_node().await?;

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
        ctx.send(msg.return_route().clone(), msg.into_body()?).await
    }
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn simple_worker__run_node_lifecycle__should_not_fail(ctx: &mut Context) -> Result<()> {
    let initialize_was_called = Arc::new(AtomicBool::new(false));
    let shutdown_was_called = Arc::new(AtomicBool::new(false));

    let initialize_was_called_clone = initialize_was_called.clone();
    let shutdown_was_called_clone = shutdown_was_called.clone();

    let worker = SimpleWorker {
        initialize_was_called: initialize_was_called_clone,
        shutdown_was_called: shutdown_was_called_clone,
    };

    ctx.start_worker("simple_worker", worker)?;

    let msg: String = ctx
        .send_and_receive(route!["simple_worker"], "Hello".to_string())
        .await?;
    assert_eq!(msg, "Hello");

    Ok(())
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

    ctx.start_worker("simple_worker", worker)?;

    let msg: String = ctx
        .send_and_receive(route!["simple_worker"], "Hello".to_string())
        .await?;
    assert_eq!(msg, "Hello");

    ctx.shutdown_node().await?;
    // Wait till tokio Runtime is shut down
    sleep(Duration::new(1, 0)).await;

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(shutdown_was_called.load(Ordering::Relaxed));

    Ok(())
}

struct FailingWorkerProcessor {
    shutdown_was_called: Arc<AtomicBool>,
}

#[async_trait]
impl Worker for FailingWorkerProcessor {
    type Context = Context;
    type Message = String;

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Err(ockam_core::Error::new(Origin::Core, Kind::Internal, "test"))
    }

    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.shutdown_was_called.store(true, Ordering::Relaxed);
        Ok(())
    }

    async fn handle_message(
        &mut self,
        _ctx: &mut Self::Context,
        _msg: Routed<Self::Message>,
    ) -> Result<()> {
        Ok(())
    }
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn worker_initialize_fail_should_shutdown(ctx: &mut Context) -> Result<()> {
    let shutdown_was_called = Arc::new(AtomicBool::new(false));
    let address = Address::from_string("failing_worker");
    let worker = FailingWorkerProcessor {
        shutdown_was_called: shutdown_was_called.clone(),
    };
    let res = ctx.start_worker(address.clone(), worker);
    assert!(res.is_ok());
    sleep(Duration::new(1, 0)).await;
    assert!(shutdown_was_called.load(Ordering::Relaxed));

    assert!(!ctx.list_workers()?.contains(&address));

    Ok(())
}

#[async_trait]
impl Processor for FailingWorkerProcessor {
    type Context = Context;

    async fn process(&mut self, _ctx: &mut Self::Context) -> Result<bool> {
        Ok(true)
    }

    async fn initialize(&mut self, _context: &mut Self::Context) -> Result<()> {
        Err(ockam_core::Error::new(Origin::Core, Kind::Internal, "test"))
    }

    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.shutdown_was_called.store(true, Ordering::Relaxed);
        Ok(())
    }
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn processor_initialize_fail_should_shutdown(ctx: &mut Context) -> Result<()> {
    let shutdown_was_called = Arc::new(AtomicBool::new(false));
    let address = Address::from_string("failing_processor");
    let processor = FailingWorkerProcessor {
        shutdown_was_called: shutdown_was_called.clone(),
    };
    let res = ctx.start_processor(address.clone(), processor);
    assert!(res.is_ok());
    sleep(Duration::new(1, 0)).await;
    assert!(shutdown_was_called.load(Ordering::Relaxed));
    assert!(!ctx.list_workers()?.contains(&address));

    Ok(())
}

struct DummyProcessor;

#[async_trait]
impl Processor for DummyProcessor {
    type Context = Context;

    async fn process(&mut self, _ctx: &mut Context) -> Result<bool> {
        tokio::task::yield_now().await;
        Ok(true)
    }
}

#[ockam_macros::test]
async fn starting_processor_with_dup_address_should_fail(ctx: &mut Context) -> Result<()> {
    ctx.start_processor("dummy_processor", DummyProcessor)?;
    assert!(ctx
        .start_processor("dummy_processor", DummyProcessor)
        .is_err());
    Ok(())
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

    ctx.start_processor("counting_processor", processor)?;
    sleep(Duration::from_secs(1)).await;

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(shutdown_was_called.load(Ordering::Relaxed));
    assert_eq!(5, run_called_count.load(Ordering::Relaxed));

    Ok(())
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
        Ok(())
    }

    async fn shutdown(&mut self, _context: &mut Self::Context) -> Result<()> {
        self.shutdown_was_called.store(true, Ordering::Relaxed);
        assert!(self.initialize_was_called.load(Ordering::Relaxed));
        Ok(())
    }

    async fn process(&mut self, _ctx: &mut Self::Context) -> Result<bool> {
        sleep(Duration::from_secs(10)).await;
        Ok(true)
    }
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn waiting_processor__shutdown__should_be_interrupted(ctx: &mut Context) -> Result<()> {
    let initialize_was_called = Arc::new(AtomicBool::new(false));
    let shutdown_was_called = Arc::new(AtomicBool::new(false));

    let processor = WaitingProcessor {
        initialize_was_called: initialize_was_called.clone(),
        shutdown_was_called: shutdown_was_called.clone(),
    };

    ctx.start_processor("waiting_processor", processor)?;
    sleep(Duration::from_secs(1)).await;

    ctx.stop_address(&"waiting_processor".into())?;
    sleep(Duration::from_secs(1)).await;

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(shutdown_was_called.load(Ordering::Relaxed));
    Ok(())
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
        let route = msg.return_route().clone();
        let body = msg.into_body()?;

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

    ctx.start_processor_with_access_control("messaging_processor", processor, AllowAll, AllowAll)?;
    sleep(Duration::from_millis(250)).await;

    let msg: String = ctx
        .send_and_receive(route!["messaging_processor"], "Keep working".to_string())
        .await?;
    assert_eq!("OK", msg);

    sleep(Duration::from_millis(250)).await;

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(!shutdown_was_called.load(Ordering::Relaxed));

    let msg: String = ctx
        .send_and_receive(route!["messaging_processor"], "Stop working".to_string())
        .await?;
    assert_eq!("I go home", msg);

    sleep(Duration::from_millis(250)).await;

    assert!(initialize_was_called.load(Ordering::Relaxed));
    assert!(shutdown_was_called.load(Ordering::Relaxed));

    Ok(())
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
    ctx.start_worker_with_access_control("bad", BadWorker, DenyAll, DenyAll)?;

    ockam_node::tokio::time::timeout(Duration::from_secs(2), ctx.shutdown_node())
        .await
        .unwrap()
}

struct SendReceiveWorker;

#[async_trait]
impl Worker for SendReceiveWorker {
    type Context = Context;
    type Message = Any;

    async fn handle_message(&mut self, ctx: &mut Context, msg: Routed<Any>) -> Result<()> {
        let return_route = msg.return_route().clone();
        let msg = SendReceiveRequest::decode(msg.payload())?;

        match msg {
            SendReceiveRequest::Connect() => {
                ctx.send(return_route, SendReceiveResponse::Connect(Ok(())))
                    .await?;
            }
        }

        ctx.shutdown_node().await
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
    ctx.start_worker("SendReceiveWorker", SendReceiveWorker)?;

    let msg_tx = SendReceiveRequest::Connect();
    let msg_rx = ctx.send_and_receive("SendReceiveWorker", msg_tx).await?;

    if let SendReceiveResponse::Connect(Err(e)) = msg_rx {
        panic!("test failure: {}", e)
    }
    Ok(())
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
        ctx.send(msg.return_route().clone(), msg.into_body()?).await
    }
}

#[ockam_macros::test]
async fn starting_worker_with_dup_address_should_fail(ctx: &mut Context) -> Result<()> {
    ctx.start_worker_with_access_control("dummy_worker", DummyWorker, DenyAll, DenyAll)?;
    assert!(ctx
        .start_worker_with_access_control("dummy_worker", DummyWorker, DenyAll, DenyAll)
        .is_err());
    Ok(())
}

struct CountingErrorWorker {
    pub(crate) counter: Arc<AtomicI8>,
}

#[async_trait]
impl Worker for CountingErrorWorker {
    type Context = Context;
    type Message = Any;

    async fn handle_message(
        &mut self,
        _context: &mut Self::Context,
        _msg: Routed<Self::Message>,
    ) -> Result<()> {
        let _ = self.counter.fetch_add(1, Ordering::Relaxed);

        Err(ockam_core::Error::new(Origin::Core, Kind::Misuse, "test"))
    }
}

#[allow(non_snake_case)]
#[ockam_macros::test]
async fn message_handle__error_during_handling__keep_worker_running(
    ctx: &mut Context,
) -> Result<()> {
    let counter = Arc::new(AtomicI8::new(0));
    ctx.start_worker(
        "counter",
        CountingErrorWorker {
            counter: counter.clone(),
        },
    )?;

    ctx.send("counter", "test".to_string()).await?;
    ctx.sleep(Duration::from_millis(100)).await;
    assert_eq!(1, counter.load(Ordering::Relaxed));

    ctx.send("counter", "test".to_string()).await?;
    ctx.sleep(Duration::from_millis(100)).await;
    assert_eq!(2, counter.load(Ordering::Relaxed));

    ctx.send("counter", "test".to_string()).await?;
    ctx.sleep(Duration::from_millis(100)).await;
    assert_eq!(3, counter.load(Ordering::Relaxed));

    Ok(())
}
