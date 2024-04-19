use core::sync::atomic::Ordering;
use core::time::Duration;

use ockam_core::{Message, RelayMessage, Result, Routed};

use crate::debugger;
use crate::error::*;
use crate::tokio::time::timeout;
use crate::{Context, DEFAULT_TIMEOUT};

pub(super) enum MessageWait {
    Timeout(Duration),
    Blocking,
}

/// Full set of options to `send_and_receive_extended` function
pub struct MessageReceiveOptions {
    message_wait: MessageWait,
}

impl Default for MessageReceiveOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageReceiveOptions {
    /// Default options with [`DEFAULT_TIMEOUT`]
    pub fn new() -> Self {
        Self {
            message_wait: MessageWait::Timeout(DEFAULT_TIMEOUT),
        }
    }

    pub(super) fn with_message_wait(mut self, message_wait: MessageWait) -> Self {
        self.message_wait = message_wait;
        self
    }

    /// Set custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.message_wait = MessageWait::Timeout(timeout);
        self
    }

    /// Set custom timeout in seconds
    pub fn with_timeout_secs(mut self, timeout: u64) -> Self {
        self.message_wait = MessageWait::Timeout(Duration::from_secs(timeout));
        self
    }

    /// Wait for the message forever
    pub fn without_timeout(mut self) -> Self {
        self.message_wait = MessageWait::Blocking;
        self
    }
}

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
                    relay_msg.source(),
                    relay_msg.destination()
                );
                debug!(
                    "Message return_route: {:?} onward_route: {:?}",
                    relay_msg.return_route(),
                    relay_msg.onward_route()
                );
                continue;
            }

            return Ok(Some(relay_msg));
        }
    }

    /// A convenience function to get a Routed message from the Mailbox
    async fn next_from_mailbox<M: Message>(&mut self) -> Result<Routed<M>> {
        let msg = self
            .receiver_next()
            .await?
            .ok_or_else(|| NodeError::Data.not_found())?;
        let destination_addr = msg.destination().clone();
        let src_addr = msg.source().clone();
        let local_msg = msg.into_local_message();

        Ok(Routed::new(destination_addr, src_addr, local_msg))
    }

    /// Block the current worker to wait for a typed message
    ///
    /// This function may return a `Err(FailedLoadData)` if the
    /// underlying worker was shut down, or `Err(Timeout)` if the call
    /// was waiting for longer than the `default timeout`.
    ///
    /// Use [`receive_extended()`](Self::receive_extended) to use a specific timeout period.
    ///
    /// Will return `None` if the corresponding worker has been
    /// stopped, or the underlying Node has shut down.
    pub async fn receive<M: Message>(&mut self) -> Result<Routed<M>> {
        self.receive_extended(MessageReceiveOptions::new()).await
    }

    /// Wait to receive a typed message
    pub async fn receive_extended<M: Message>(
        &mut self,
        options: MessageReceiveOptions,
    ) -> Result<Routed<M>> {
        match options.message_wait {
            MessageWait::Timeout(timeout_duration) => {
                timeout(timeout_duration, async { self.next_from_mailbox().await })
                    .await
                    .map_err(|e| NodeError::Data.with_elapsed(e))?
            }
            MessageWait::Blocking => self.next_from_mailbox().await,
        }
    }
}
