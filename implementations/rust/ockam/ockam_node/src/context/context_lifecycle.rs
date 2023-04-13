use core::time::Duration;

use ockam_core::compat::collections::HashMap;
use ockam_core::compat::{boxed::Box, sync::Arc, sync::RwLock};
use ockam_core::flow_control::FlowControls;
use ockam_core::{
    errcode::{Kind, Origin},
    Address, AsyncTryClone, DenyAll, Error, IncomingAccessControl, Mailboxes,
    OutgoingAccessControl, Result, TransportType,
};
use ockam_transport_core::Transport;

use crate::async_drop::AsyncDrop;
use crate::channel_types::{message_channel, small_channel, SmallReceiver, SmallSender};
use crate::tokio::{self, runtime::Handle};
use crate::{debugger, Context};
use crate::{error::*, relay::CtrlSignal, router::SenderPair, NodeMessage};

/// A special type of `Context` that has no worker relay and inherits
/// the parent `Context`'s access control
pub type DetachedContext = Context;

/// A special sender type that connects a type to an AsyncDrop handler
pub type AsyncDropSender = crate::tokio::sync::oneshot::Sender<Address>;

impl Drop for Context {
    fn drop(&mut self) {
        if let Some(sender) = self.async_drop_sender.take() {
            trace!("De-allocated detached context {}", self.address());
            if let Err(e) = sender.send(self.address()) {
                warn!("Encountered error while dropping detached context: {}", e);
            }
        }
    }
}

#[ockam_core::async_trait]
impl AsyncTryClone for Context {
    async fn async_try_clone(&self) -> Result<Self> {
        // TODO: @ac ignores parent Access Control. Should be documented somewhere
        self.new_detached(
            Address::random_tagged("Context.async_try_clone.detached"),
            DenyAll,
            DenyAll,
        )
        .await
    }
}

impl Context {
    /// Create a new context
    ///
    /// This function returns a new instance of Context, the relay
    /// sender pair, and relay control signal receiver.
    ///
    /// `async_drop_sender` must be provided when creating a detached
    /// Context type (i.e. not backed by a worker relay).
    pub(crate) fn new(
        rt: Handle,
        sender: SmallSender<NodeMessage>,
        mailboxes: Mailboxes,
        async_drop_sender: Option<AsyncDropSender>,
        transports: Arc<RwLock<HashMap<TransportType, Arc<dyn Transport>>>>,
        flow_controls: &FlowControls,
    ) -> (Self, SenderPair, SmallReceiver<CtrlSignal>) {
        let (mailbox_tx, receiver) = message_channel();
        let (ctrl_tx, ctrl_rx) = small_channel();
        (
            Self {
                rt,
                sender,
                mailboxes,
                receiver,
                async_drop_sender,
                mailbox_count: Arc::new(0.into()),
                transports,
                flow_controls: flow_controls.clone(),
            },
            SenderPair {
                msgs: mailbox_tx,
                ctrl: ctrl_tx,
            },
            ctrl_rx,
        )
    }

    pub(crate) fn copy_with_mailboxes(
        &self,
        mailboxes: Mailboxes,
    ) -> (Context, SenderPair, SmallReceiver<CtrlSignal>) {
        Context::new(
            self.runtime().clone(),
            self.sender().clone(),
            mailboxes,
            None,
            self.transports.clone(),
            &self.flow_controls,
        )
    }

    pub(crate) fn copy_with_mailboxes_detached(
        &self,
        mailboxes: Mailboxes,
        drop_sender: AsyncDropSender,
    ) -> (Context, SenderPair, SmallReceiver<CtrlSignal>) {
        Context::new(
            self.runtime().clone(),
            self.sender().clone(),
            mailboxes,
            Some(drop_sender),
            self.transports.clone(),
            &self.flow_controls,
        )
    }

    /// Utility function to sleep tasks from other crates
    #[doc(hidden)]
    pub async fn sleep(&self, dur: Duration) {
        tokio::time::sleep(dur).await;
    }

    /// TODO basically we can just rename `Self::new_detached_impl()`
    pub async fn new_detached_with_mailboxes(
        &self,
        mailboxes: Mailboxes,
    ) -> Result<DetachedContext> {
        let ctx = self.new_detached_impl(mailboxes).await?;

        debugger::log_inherit_context("DETACHED_WITH_MB", self, &ctx);

        Ok(ctx)
    }

    /// Create a new detached `Context` without spawning a full worker
    ///
    /// Note: this function is very low-level.  For most users
    /// [`start_worker()`](Self::start_worker) is the recommended way
    /// to create a new worker context.
    ///
    pub async fn new_detached(
        &self,
        address: impl Into<Address>,
        incoming: impl IncomingAccessControl,
        outgoing: impl OutgoingAccessControl,
    ) -> Result<DetachedContext> {
        let mailboxes = Mailboxes::main(address.into(), Arc::new(incoming), Arc::new(outgoing));
        let ctx = self.new_detached_impl(mailboxes).await?;

        debugger::log_inherit_context("DETACHED", self, &ctx);

        Ok(ctx)
    }

    async fn new_detached_impl(&self, mailboxes: Mailboxes) -> Result<DetachedContext> {
        // A detached Context exists without a worker relay, which
        // requires special shutdown handling.  To allow the Drop
        // handler to interact with the Node runtime, we use an
        // AsyncDrop handler.
        //
        // This handler is spawned and listens for an event from the
        // Drop handler, and then forwards a message to the Node
        // router.
        let (async_drop, drop_sender) = AsyncDrop::new(self.sender.clone());
        self.rt.spawn(async_drop.run());

        // Create a new context and get access to the mailbox senders
        let addresses = mailboxes.addresses();
        let (ctx, sender, _) = self.copy_with_mailboxes_detached(mailboxes, drop_sender);

        // Create a "detached relay" and register it with the router
        let (msg, mut rx) =
            NodeMessage::start_worker(addresses, sender, true, Arc::clone(&self.mailbox_count));
        self.sender
            .send(msg)
            .await
            .map_err(|e| Error::new(Origin::Node, Kind::Invalid, e))?;
        rx.recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??;

        Ok(ctx)
    }
}

#[cfg(test)]
mod tests {
    use ockam_core::{async_trait, Mailbox};

    use super::*;

    #[ockam_macros::test(crate = "crate")]
    async fn test_copy(ctx: &mut Context) -> Result<()> {
        let transport = Arc::new(SomeTransport());
        ctx.register_transport(transport.clone());

        // after a copy with new mailboxes the list of transports should be intact
        let mailboxes = Mailboxes::new(Mailbox::deny_all("address"), vec![]);
        let (copy, _, _) = ctx.copy_with_mailboxes(mailboxes.clone());
        assert!(copy.is_transport_registered(transport.transport_type()));

        // after a detached copy with new mailboxes the list of transports should be intact
        let (_, drop_sender) = AsyncDrop::new(ctx.sender.clone());
        let (copy, _, _) = ctx.copy_with_mailboxes_detached(mailboxes, drop_sender);
        assert!(copy.is_transport_registered(transport.transport_type()));

        ctx.stop().await
    }

    struct SomeTransport();

    #[async_trait]
    impl Transport for SomeTransport {
        fn transport_type(&self) -> TransportType {
            TransportType::new(0)
        }

        async fn resolve_address(&self, address: Address) -> Result<Address> {
            Ok(address)
        }
    }
}
