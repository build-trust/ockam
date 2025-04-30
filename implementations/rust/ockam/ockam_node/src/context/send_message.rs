use crate::error::*;
use crate::router::Router;
use crate::tokio::runtime::Handle;
use crate::{debugger, Context, MessageSendOptions, MessageSendReceiveOptions};

use core::sync::atomic::AtomicUsize;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::flow_control::FlowControls;
use ockam_core::{
    errcode::{Kind, Origin},
    Address, Error, LocalMessage, Mailboxes, Message, RelayMessage, Result, Route, Routed,
    TransportType,
};
use ockam_core::{AllowAll, Mailbox};
use ockam_transport_core::Transport;

#[cfg(feature = "std")]
use ockam_core::OpenTelemetryContext;

impl Context {
    /// Using a temporary new context, send a message and then receive a message
    /// with default timeout and no flow control
    ///
    /// This helper function uses [`new_detached`], [`send`], and
    /// [`receive`] internally. See their documentation for more
    /// details.
    ///
    /// [`new_detached`]: Self::new_detached
    /// [`send`]: Self::send
    /// [`receive`]: Self::receive
    pub async fn send_and_receive<T, R>(&self, route: impl Into<Route>, msg: T) -> Result<R>
    where
        T: Message,
        R: Message,
    {
        self.send_and_receive_extended::<T, R>(route, msg, MessageSendReceiveOptions::new())
            .await?
            .into_body()
    }

    /// Using a temporary new context, send a message
    ///
    /// This helper function uses [`new_detached`] and [`send`] internally.
    /// See their documentation for more details.
    ///
    /// [`new_detached`]: Self::new_detached
    /// [`send`]: Self::send
    pub async fn send_extended<T>(
        &self,
        route: impl Into<Route>,
        msg: T,
        options: MessageSendOptions,
    ) -> Result<()>
    where
        T: Message,
    {
        self.state.send(route.into(), msg, options).await
    }
    /// Using a temporary new context, send a message and then receive a message
    ///
    /// This helper function uses [`new_detached`], [`send`], and
    /// [`receive`] internally. See their documentation for more
    /// details.
    ///
    /// [`new_detached`]: Self::new_detached
    /// [`send`]: Self::send
    /// [`receive`]: Self::receive
    pub async fn send_and_receive_extended<T, R>(
        &self,
        route: impl Into<Route>,
        msg: T,
        options: MessageSendReceiveOptions,
    ) -> Result<Routed<R>>
    where
        T: Message,
        R: Message,
    {
        Self::send_and_receive_extended_impl(
            self.runtime().clone(),
            self.router()?,
            self.transports.clone(),
            self.flow_controls(),
            self.mailbox_count(),
            route.into(),
            msg,
            options,
            #[cfg(feature = "std")]
            self.tracing_context(),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn send_and_receive_extended_impl<T, R>(
        runtime: Handle,
        router: Arc<Router>,
        transports: Arc<RwLock<HashMap<TransportType, Arc<dyn Transport>>>>,
        flow_controls: &FlowControls,
        mailbox_count: Arc<AtomicUsize>,
        route: Route,
        msg: T,
        options: MessageSendReceiveOptions,
        #[cfg(feature = "std")] tracing_context: OpenTelemetryContext,
    ) -> Result<Routed<R>>
    where
        T: Message,
        R: Message,
    {
        let mut child_ctx = Self::make_send_and_receive_context(
            runtime,
            router,
            transports,
            flow_controls,
            mailbox_count,
            route.next()?.clone(),
            #[cfg(feature = "std")]
            tracing_context,
        )
        .await?;
        let (send, receive) = options.dissolve();
        child_ctx.send_extended(route, msg, send).await?;
        child_ctx.receive_extended::<R>(receive).await
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn make_send_and_receive_context(
        runtime: Handle,
        router: Arc<Router>,
        transports: Arc<RwLock<HashMap<TransportType, Arc<dyn Transport>>>>,
        flow_controls: &FlowControls,
        mailbox_count: Arc<AtomicUsize>,
        next: Address,
        #[cfg(feature = "std")] tracing_context: OpenTelemetryContext,
    ) -> Result<Context> {
        let address = Address::random_tagged("Context.send_and_receive.detached");

        let mailboxes = Mailboxes::new(
            Mailbox::new(
                address.clone(),
                None,
                Arc::new(AllowAll),
                Arc::new(AllowAll),
            ),
            vec![],
        );

        if let Some(flow_control_id) = flow_controls
            .find_flow_control_with_producer_address(&next)
            .map(|x| x.flow_control_id().clone())
        {
            // To be able to receive the response
            flow_controls.add_consumer(&address, &flow_control_id);
        }

        let child_ctx = Self::new_detached_with_mailboxes_impl(
            runtime,
            router,
            transports,
            flow_controls,
            mailboxes,
            mailbox_count,
        )?;

        #[cfg(feature = "std")]
        child_ctx.set_tracing_context(tracing_context);
        Ok(child_ctx)
    }

    /// Send a message to an address or via a fully-qualified route
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`Address`]: ockam_core::Address
    /// [`RouteBuilder`]: ockam_core::RouteBuilder
    ///
    /// ```rust
    /// # use {ockam_node::Context, ockam_core::Result};    /// #
    /// use ockam_core::{deserialize, serialize, Decodable, Encodable, Encoded};
    ///
    /// async fn test(ctx: &mut Context) -> Result<()> {
    /// use ockam_core::Message;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Message, Serialize, Deserialize)]
    /// struct MyMessage(String);
    ///
    /// impl MyMessage {
    ///     fn new(s: &str) -> Self {
    ///         Self(s.into())
    ///     }
    /// }
    ///
    /// impl Encodable for MyMessage {
    ///     fn encode(self) -> Result<Encoded> {
    ///         Ok(serialize(self)?)
    ///     }
    /// }
    ///
    /// impl Decodable for MyMessage {
    ///     fn decode(e: &[u8]) -> Result<Self> {
    ///         Ok(deserialize(e)?)
    ///     }
    /// }
    ///
    /// ctx.send("my-test-worker", MyMessage::new("Hello you there :)")).await?;
    /// Ok(())
    /// # }
    /// ```
    pub async fn send<R, M>(&self, route: R, msg: M) -> Result<()>
    where
        R: Into<Route>,
        M: Message,
    {
        self.send_extended(route.into(), msg, MessageSendOptions::new())
            .await
    }

    /// Forward a transport message to its next routing destination
    ///
    /// Similar to [`Context::send`], but taking a
    /// [`LocalMessage`], which contains the full destination
    /// route, and calculated return route for this hop.
    ///
    /// **Note:** you most likely want to use
    /// [`Context::send`] instead, unless you are writing an
    /// external router implementation for ockam node.
    ///
    /// [`Context::send`]: crate::Context::send
    /// [`LocalMessage`]: ockam_core::LocalMessage
    pub async fn forward(&self, local_msg: LocalMessage) -> Result<()> {
        self.forward_from_address(local_msg, self.primary_address().clone())
            .await
    }

    /// Forward a transport message to its next routing destination
    ///
    /// Similar to [`Context::send`], but taking a
    /// [`LocalMessage`], which contains the full destination
    /// route, and calculated return route for this hop.
    ///
    /// **Note:** you most likely want to use
    /// [`Context::send`] instead, unless you are writing an
    /// external router implementation for ockam node.
    ///
    /// [`Context::send`]: crate::Context::send
    /// [`LocalMessage`]: ockam_core::LocalMessage
    pub async fn forward_from_address(
        &self,
        local_msg: LocalMessage,
        sending_address: Address,
    ) -> Result<()> {
        // Check if the sender address exists
        if !self.mailboxes().contains(&sending_address) {
            return Err(Error::new_without_cause(Origin::Node, Kind::Invalid));
        }

        // First resolve the next hop in the route
        let addr = match local_msg.onward_route().next() {
            Ok(next) => next.clone(),
            Err(err) => {
                // TODO: communicate bad routes to calling function
                error!(
                    "Invalid onward route for message forwarded from {}",
                    local_msg.return_route()
                );
                return Err(err);
            }
        };
        let sender = self.router()?.resolve(&addr)?;

        // Pack the transport message into a RelayMessage wrapper
        let relay_msg = RelayMessage::new(sending_address, addr, local_msg);

        debugger::log_outgoing_message(self.primary_address(), &relay_msg);

        if !self.mailboxes().is_outgoing_authorized(&relay_msg).await? {
            warn!(
                "Message forwarded from {} to {} did not pass outgoing access control",
                relay_msg.source(),
                relay_msg.destination(),
            );
            return Ok(());
        }

        // Forward the message
        sender
            .send(relay_msg)
            .await
            .map_err(NodeError::from_send_err)?;

        Ok(())
    }

    /// Send a message to an address or via a fully-qualified route
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`Address`]: ockam_core::Address
    /// [`RouteBuilder`]: ockam_core::RouteBuilder
    ///
    /// This function additionally takes the sending address
    /// parameter, to specify which of a worker's (or processor's)
    /// addresses should be used.
    pub async fn send_from_address<R, M>(
        &self,
        route: R,
        msg: M,
        sending_address: Address,
    ) -> Result<()>
    where
        R: Into<Route>,
        M: Message,
    {
        self.state
            .send(
                route.into(),
                msg,
                MessageSendOptions::new().with_sending_address(sending_address),
            )
            .await
    }
}
