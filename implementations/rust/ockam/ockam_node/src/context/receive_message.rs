use crate::debugger;
use crate::tokio::time::timeout;
use crate::{error::*, parser};
use crate::{Context, DEFAULT_TIMEOUT};
use core::sync::atomic::Ordering;
use core::time::Duration;
use ockam_core::{Message, RelayMessage, Result, Routed};

impl Context {
    /// Wait for the next message from the mailbox
    pub(crate) async fn receiver_next(&mut self) -> Result<Option<RelayMessage>> {
        loop {
            let relay_msg = if let Some(msg) = self.receiver.recv().await.map(|msg| {
                trace!("{}: received new message!", self.address());

                // First we update the mailbox fill metrics
                self.mailbox_count.fetch_sub(1, Ordering::Acquire);

                msg
            }) {
                msg
            } else {
                // no more messages
                return Ok(None);
            };

            debugger::log_incoming_message(self, &relay_msg);

            if !self.mailboxes.is_incoming_authorized(&relay_msg).await? {
                warn!(
                    "Message received from {} for {} did not pass incoming access control",
                    relay_msg.return_route(),
                    relay_msg.destination()
                );
                continue;
            }

            return Ok(Some(relay_msg));
        }
    }

    /// This function will block and re-queue messages into the
    /// mailbox until it can receive the correct message payload.
    ///
    /// WARNING: this will temporarily create a busyloop, this
    /// mechanism should be replaced with a waker system that lets the
    /// mailbox work not yield another message until the relay worker
    /// has woken it.
    /// A convenience function to get a Routed message from the Mailbox
    async fn next_from_mailbox<M: Message>(&mut self) -> Result<Routed<M>> {
        loop {
            let msg = self
                .receiver_next()
                .await?
                .ok_or_else(|| NodeError::Data.not_found())?;
            let destination_addr = msg.destination().clone();
            let src_addr = msg.source().clone();
            let local_msg = msg.into_local_message();

            // FIXME: make message parsing idempotent to avoid cloning
            match parser::message(&local_msg.transport().payload).ok() {
                Some(msg) => break Ok((msg, local_msg, addr)),
                None => {
                    // Requeue
                    self.forward(local_msg).await?;
                }
            }
        }
    }

    /// Block the current worker to wait for a typed message
    ///
    /// **Warning** this function will wait until its running ockam
    /// node is shut down.  A safer variant of this function is
    /// [`receive`](Self::receive) and
    /// [`receive_timeout`](Self::receive_timeout).
    pub async fn receive_block<M: Message>(&mut self) -> Result<Routed<M>> {
        self.next_from_mailbox().await
    }

    /// Block the current worker to wait for a typed message
    ///
    /// This function may return a `Err(FailedLoadData)` if the
    /// underlying worker was shut down, or `Err(Timeout)` if the call
    /// was waiting for longer than the `default timeout`.  Use
    /// [`receive_timeout`](Context::receive_timeout) to adjust the
    /// timeout period.
    ///
    /// Will return `None` if the corresponding worker has been
    /// stopped, or the underlying Node has shut down.
    pub async fn receive<M: Message>(&mut self) -> Result<Routed<M>> {
        self.receive_timeout(DEFAULT_TIMEOUT).await
    }

    /// Wait to receive a message up to a specified timeout
    ///
    /// See [`receive`](Self::receive) for more details.
    pub async fn receive_duration_timeout<M: Message>(
        &mut self,
        timeout_duration: Duration,
    ) -> Result<Routed<M>> {
        timeout(timeout_duration, async { self.next_from_mailbox().await })
            .await
            .map_err(|e| NodeError::Data.with_elapsed(e))?
    }

    /// Wait to receive a message up to a specified timeout
    ///
    /// See [`receive`](Self::receive) for more details.
    pub async fn receive_timeout<M: Message>(&mut self, timeout_secs: u64) -> Result<Routed<M>> {
        self.receive_duration_timeout(Duration::from_secs(timeout_secs))
            .await
    }
}
