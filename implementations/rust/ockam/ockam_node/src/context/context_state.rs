use crate::router::Router;
use crate::{debugger, NodeError};
use core::sync::atomic::AtomicUsize;
use ockam_core::compat::sync::{Arc, Weak};
use ockam_core::compat::vec::Vec;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::flow_control::FlowControls;
use ockam_core::Result;
use ockam_core::{
    route, Address, Error, LocalInfo, LocalMessage, Mailboxes, Message, RelayMessage, Route,
};

#[cfg(feature = "std")]
use ockam_core::compat::sync::RwLock;
#[cfg(feature = "std")]
use ockam_core::OpenTelemetryContext;
#[cfg(feature = "std")]
use opentelemetry::trace::{Span, TraceContextExt};

pub(crate) struct ContextState {
    pub(super) mailboxes: Mailboxes,
    pub(super) router: Weak<Router>,
    pub(super) mailbox_count: Arc<AtomicUsize>,
    pub(super) flow_controls: FlowControls,
    #[cfg(feature = "std")]
    pub(super) tracing_context: RwLock<OpenTelemetryContext>,
}

impl ContextState {
    /// Return the primary address of the current worker
    pub fn primary_address(&self) -> &Address {
        self.mailboxes.primary_address()
    }

    /// Return a reference to the mailboxes of this context
    pub fn mailboxes(&self) -> &Mailboxes {
        &self.mailboxes
    }

    pub(super) fn router(&self) -> Result<Arc<Router>> {
        self.router
            .upgrade()
            .ok_or_else(|| Error::new(Origin::Node, Kind::Shutdown, "Failed to upgrade router"))
    }

    /// Return the tracing context
    #[cfg(feature = "std")]
    pub fn tracing_context(&self) -> OpenTelemetryContext {
        self.tracing_context.read().unwrap().clone()
    }

    /// Set the current tracing context
    #[cfg(feature = "std")]
    pub fn set_tracing_context(&self, tracing_context: OpenTelemetryContext) {
        *self.tracing_context.write().unwrap() = tracing_context
    }

    /// Set the current tracing context from a started span
    #[cfg(feature = "std")]
    pub fn set_tracing_context_from_span(&self, span: impl Span) {
        let context =
            opentelemetry::Context::new().with_remote_span_context(span.span_context().clone());
        *self.tracing_context.write().unwrap() = OpenTelemetryContext::inject(&context)
    }

    pub(super) async fn send_from_address_impl<M>(
        &self,
        route: Route,
        msg: M,
        sending_address: Address,
        local_info: Vec<LocalInfo>,
    ) -> Result<()>
    where
        M: Message,
    {
        // Check if the sender address exists
        if !self.mailboxes.contains(&sending_address) {
            return Err(Error::new_without_cause(Origin::Node, Kind::Invalid));
        }

        // First resolve the next hop in the route
        let addr = match route.next() {
            Ok(next) => next.clone(),
            Err(err) => {
                // TODO: communicate bad routes to calling function
                error!("Invalid route for message sent from {}", sending_address);
                return Err(err);
            }
        };

        let sender = self.router()?.resolve(&addr)?;

        // Pack the payload into a TransportMessage
        let payload = msg.encode().map_err(|_| NodeError::Data.internal())?;

        // Pack transport message into a LocalMessage wrapper
        let local_msg = LocalMessage::new()
            .with_onward_route(route)
            .with_return_route(route![sending_address.clone()])
            .with_payload(payload)
            .with_local_info(local_info);

        // make sure to set the latest tracing context, to get the latest span id
        #[cfg(feature = "std")]
        let local_msg = local_msg.with_tracing_context(self.tracing_context().update());

        // Pack local message into a RelayMessage wrapper
        let relay_msg = RelayMessage::new(sending_address, addr, local_msg);

        debugger::log_outgoing_message(self.mailboxes.primary_address(), &relay_msg);

        if !self.mailboxes().is_outgoing_authorized(&relay_msg).await? {
            warn!(
                "Message sent from {} to {} did not pass outgoing access control",
                relay_msg.source(),
                relay_msg.destination()
            );
            return Ok(());
        }

        // Send the packed user message with associated route
        sender
            .send(relay_msg)
            .await
            .map_err(NodeError::from_send_err)?;

        Ok(())
    }
}
