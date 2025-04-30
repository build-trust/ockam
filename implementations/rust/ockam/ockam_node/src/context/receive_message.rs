use core::sync::atomic::Ordering;

use ockam_core::compat::sync::Arc;
use ockam_core::{IncomingAccessControl, Message, RelayMessage, Result, Routed};

use crate::context::message_options::MessageWait;
use crate::error::*;
use crate::tokio::time::timeout;
use crate::Context;
use crate::{debugger, MessageReceiveOptions};

impl Context {
    /// Wait for the next message from the mailbox
    pub(crate) async fn receiver_next(
        &mut self,
        override_incoming_access_control: Option<Arc<dyn IncomingAccessControl>>,
    ) -> Result<Option<RelayMessage>> {
        loop {
            let relay_msg = if let Some(msg) = self.receiver.recv().await.map(|msg| {
                trace!(address=%self.primary_address(), "received new message!");

                // First we update the mailbox fill metrics
                self.state.mailbox_count.fetch_sub(1, Ordering::Acquire);

                msg
            }) {
                msg
            } else {
                // no more messages
                return Ok(None);
            };

            debugger::log_incoming_message(self, &relay_msg);

            if let Some(incoming_access_control) = &override_incoming_access_control {
                if !incoming_access_control.is_authorized(&relay_msg).await? {
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
            } else if !self.mailboxes().is_incoming_authorized(&relay_msg).await? {
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
    async fn next_from_mailbox<M: Message>(
        &mut self,
        override_incoming_access_control: Option<Arc<dyn IncomingAccessControl>>,
    ) -> Result<Routed<M>> {
        let msg = self
            .receiver_next(override_incoming_access_control)
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
        match options.message_wait() {
            MessageWait::Timeout(timeout_duration) => timeout(*timeout_duration, async {
                self.next_from_mailbox(options.incoming_access_control().clone())
                    .await
            })
            .await
            .map_err(|_| NodeError::Data.with_timeout(*timeout_duration))?,
            MessageWait::Blocking => {
                self.next_from_mailbox(options.incoming_access_control().clone())
                    .await
            }
        }
    }
}
