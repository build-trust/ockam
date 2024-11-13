#[cfg(not(feature = "std"))]
use crate::tokio;
use core::time::Duration;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::Weak;
use ockam_core::compat::time::now;
use ockam_core::compat::{sync::Arc, sync::RwLock};
use ockam_core::flow_control::FlowControls;
#[cfg(feature = "std")]
use ockam_core::OpenTelemetryContext;
use ockam_core::{
    Address, AllowAll, DenyAll, IncomingAccessControl, Mailbox, Mailboxes, OutgoingAccessControl,
    Result, TransportType, TryClone,
};
use ockam_transport_core::Transport;

use crate::channel_types::{message_channel, oneshot_channel, OneshotReceiver};
use crate::router::Router;
use crate::{debugger, Context, ContextMode};
use crate::{relay::CtrlSignal, router::SenderPair};
use tokio::runtime::Handle;

impl Drop for Context {
    fn drop(&mut self) {
        if let ContextMode::Detached = self.mode {
            let router = match self.router() {
                Ok(router) => router,
                Err(_) => {
                    debug!(address=%self.primary_address(), "Can't upgrade router inside Context::drop");
                    return;
                }
            };

            match router.stop_address(self.primary_address(), true) {
                Ok(_) => {
                    trace!(address=%self.primary_address(), "de-allocated detached context");
                }
                Err(err) => {
                    // That is OK during node stop
                    debug!(address=%self.primary_address(), %err, "Encountered error while dropping detached context");
                }
            }
        }
    }
}

impl TryClone for Context {
    fn try_clone(&self) -> Result<Self> {
        // TODO: @ac ignores parent Access Control. Should be documented somewhere
        self.new_detached(
            Address::random_tagged("Context.try_clone.detached"),
            DenyAll,
            DenyAll,
        )
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
    fn new(
        runtime_handle: Handle,
        router: Weak<Router>,
        mailboxes: Mailboxes,
        mode: ContextMode,
        transports: Arc<RwLock<HashMap<TransportType, Arc<dyn Transport>>>>,
        flow_controls: &FlowControls,
        #[cfg(feature = "std")] tracing_context: OpenTelemetryContext,
    ) -> (Self, SenderPair, OneshotReceiver<CtrlSignal>) {
        let (mailbox_tx, receiver) = message_channel();
        let (ctrl_tx, ctrl_rx) = oneshot_channel();
        (
            Self {
                runtime_handle,
                router,
                mailboxes,
                mode,
                receiver,
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

    pub(crate) fn create_app_context(
        runtime_handle: Handle,
        router: Weak<Router>,
        flow_controls: &FlowControls,
        #[cfg(feature = "std")] tracing_context: OpenTelemetryContext,
    ) -> (Self, SenderPair, OneshotReceiver<CtrlSignal>) {
        let addr: Address = "app".into();
        let mailboxes = Mailboxes::new(
            Mailbox::new(addr, None, Arc::new(AllowAll), Arc::new(AllowAll)),
            vec![],
        );

        Self::new(
            runtime_handle,
            router,
            mailboxes,
            ContextMode::Detached,
            Default::default(),
            flow_controls,
            #[cfg(feature = "std")]
            tracing_context,
        )
    }

    pub(crate) fn new_with_mailboxes(
        &self,
        mailboxes: Mailboxes,
        mode: ContextMode,
    ) -> (Context, SenderPair, OneshotReceiver<CtrlSignal>) {
        Self::new(
            self.runtime().clone(),
            self.router_weak(),
            mailboxes,
            mode,
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
    pub fn new_detached_with_mailboxes(&self, mailboxes: Mailboxes) -> Result<Context> {
        let ctx = self.new_detached_impl(mailboxes)?;

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
    pub fn new_detached(
        &self,
        address: impl Into<Address>,
        incoming: impl IncomingAccessControl,
        outgoing: impl OutgoingAccessControl,
    ) -> Result<Context> {
        let mailboxes = Mailboxes::primary(address.into(), Arc::new(incoming), Arc::new(outgoing));
        let ctx = self.new_detached_impl(mailboxes)?;

        debugger::log_inherit_context("DETACHED", self, &ctx);

        Ok(ctx)
    }

    fn new_detached_impl(&self, mailboxes: Mailboxes) -> Result<Context> {
        // Create a new context and get access to the mailbox senders
        let (ctx, sender, _) = self.new_with_mailboxes(mailboxes, ContextMode::Detached);

        self.router()?.add_worker(
            ctx.mailboxes(),
            sender,
            true,
            Default::default(),
            self.mailbox_count.clone(),
        )?;

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
        let (copy, _, _) = ctx.new_with_mailboxes(mailboxes.clone(), ContextMode::Attached);
        assert!(copy.is_transport_registered(transport.transport_type()));

        // after a detached copy with new mailboxes the list of transports should be intact
        let (copy, _, _) = ctx.new_with_mailboxes(mailboxes, ContextMode::Attached);
        assert!(copy.is_transport_registered(transport.transport_type()));
        Ok(())
    }

    struct SomeTransport();

    #[async_trait]
    impl Transport for SomeTransport {
        fn transport_type(&self) -> TransportType {
            TransportType::new(0)
        }

        async fn resolve_address(&self, address: &Address) -> Result<Address> {
            Ok(address.clone())
        }

        fn disconnect(&self, _address: &Address) -> Result<()> {
            Ok(())
        }
    }
}
