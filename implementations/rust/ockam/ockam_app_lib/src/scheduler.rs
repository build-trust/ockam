//! Scheduler
//! The scheduler is responsible for running operations in the background.
//! It is implemented as a Tokio task that waits for a notification to run an operation.
//! The notification can be triggered by the user or by a timeout.

use ockam_core::async_trait;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::{timeout, Duration};

#[async_trait]
pub(crate) trait ScheduledTask: Send + Sync + 'static {
    async fn run(&self);
}

#[derive(Clone)]
pub(crate) struct Scheduler {
    task: Arc<dyn ScheduledTask>,
    interval: Duration,
    tx: Sender<()>,
}

impl Scheduler {
    /// Create a new scheduler instance and spawn a Tokio task to run it
    pub(crate) fn create(
        task: Arc<dyn ScheduledTask>,
        interval: Duration,
        runtime: &Handle,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<()>(1);
        let instance = Self { tx, task, interval };
        {
            let instance = instance.clone();
            runtime.spawn(async move {
                instance.start(rx).await;
            });
        }

        instance
    }

    /// Schedule the task to run immediately
    pub(crate) fn schedule_now(&self) {
        // try to send the event, ignore if the channel is full (already scheduled)
        let _ = self.tx.try_send(());
    }

    async fn start(self, mut rx: Receiver<()>) {
        loop {
            // run the task at right away to avoid extra delays in initialization phase
            self.task.run().await;

            // resets notification right after execution to avoid running the task twice
            let _ = rx.try_recv();

            // either we received an event or timed out
            let _ = timeout(self.interval, rx.recv()).await;

            // sometimes we want to refresh a state that was just updated and we end up
            // fetching the previous status instead, this is a workaround to avoid that
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
