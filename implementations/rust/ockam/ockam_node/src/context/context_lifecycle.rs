use core::time::Duration;

#[cfg(not(feature = "std"))]
use crate::tokio;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::time::now;
use ockam_core::compat::{boxed::Box, sync::Arc, sync::RwLock};
use ockam_core::flow_control::FlowControls;
#[cfg(feature = "std")]
use ockam_core::OpenTelemetryContext;
use ockam_core::{
    errcode::{Kind, Origin},
    Address, AsyncTryClone, DenyAll, Error, IncomingAccessControl, Mailboxes,
    OutgoingAccessControl, Result, TransportType,
};
use ockam_transport_core::Transport;

use tokio::runtime::Handle;

use crate::async_drop::AsyncDrop;
use crate::channel_types::{message_channel, small_channel, SmallReceiver, SmallSender};
use crate::{debugger, Context};
use crate::{error::*, relay::CtrlSignal, router::SenderPair, NodeMessage};

/// A special type of `Context` that has no worker relay and inherits
/// the parent `Context`'s access control
pub type DetachedContext = Context;

/// A special sender type that connects a type to an AsyncDrop handler
pub type AsyncDropSender = tokio::sync::oneshot::Sender<Address>;

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
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        rt: Handle,
        sender: SmallSender<NodeMessage>,
        mailboxes: Mailboxes,
        async_drop_sender: Option<AsyncDropSender>,
        transports: Arc<RwLock<HashMap<TransportType, Arc<dyn Transport>>>>,
        flow_controls: &FlowControls,
        #[cfg(feature = "std")] tracing_context: OpenTelemetryContext,
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
                #[cfg(feature = "std")]
                tracing_context,
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
            #[cfg(feature = "std")]
            self.tracing_context(),
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
            #[cfg(feature = "std")]
            OpenTelemetryContext::current(),
        )
    }

    /// Utility function to sleep tasks from other crates
    #[doc(hidden)]
    pub async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }

    /// Utility function to sleep tasks for long periods of time (seconds precision)
    /// Difference between this and `sleep` is that this sleeps in 1 second intervals and recalculates time left,
    /// which account for the time the device was in sleep state
    #[doc(hidden)]
    pub async fn sleep_long_until(&self, deadline_timestamp_seconds: u64) {
        let n = now().unwrap();

        if deadline_timestamp_seconds <= n {
            return;
        }

        let duration = deadline_timestamp_seconds - n;

        if duration < 5 {
            warn!(
                "Low precision sleeping for less than 5 seconds. Duration: {:?}",
                duration
            );
            self.sleep(Duration::from_secs(duration)).await;
            return;
        }

        loop {
            self.sleep(Duration::from_secs(1)).await;
            if now().unwrap() >= deadline_timestamp_seconds {
                return;
            }
        }
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
    /// Approximate flow of starting a detached address:
    ///
    /// 1. Create and Spawn AsyncDrop::run
    /// 2. StartWorker message -> Router
    /// 3. First address is considered a primary_addr (main_addr)
    /// 4. Check if router.map.address_records_map already has primary_addr
    /// 5. AddressRecord is created and inserted in router.map
    /// 6. Iterate over metadata:
    ///     Check if it belongs to that record
    ///     Set is_terminal true in router.map.address_metadata_map (if address is terminal)
    ///     Insert attributes one by one
    /// 7. For each address we insert pair (Address, primary_addr) into router.map.alias_map, including (primary_addr, primary_addr itself)
    ///
    /// Approximate flow of stopping a detached address:
    ///
    /// 1. Context::Drop is called when Context is dropped by rust runtime (according to RAII principle)
    /// 2. async_drop_sender is used to send the Context address
    /// 3. AsyncDrop sends StopWorker message -> Router
    /// 4. Get AddressRecord
    /// 5. router.map.free_address(main_address) is called (given Router state is running):
    ///     remote main_address from router.map.stopping (it's not their anyway, unless in was a cluster and node was shutting down)
    ///     Remove AddressRecord from router.map.address_records_map (return error if not found)
    ///     Remove all alias in router.map.alias_map
    ///     Remote all meta from router.map.address_metadata
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
        let (msg, mut rx) = NodeMessage::start_worker(
            addresses,
            sender,
            true,
            Arc::clone(&self.mailbox_count),
            vec![],
        );
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
        Ok(())
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

        async fn disconnect(&self, _address: Address) -> Result<()> {
            Ok(())
        }
    }
}
