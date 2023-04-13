use crate::channel_types::small_channel;
use crate::context::MessageWait;
use crate::{debugger, Context, MessageReceiveOptions, DEFAULT_TIMEOUT};
use crate::{error::*, NodeMessage};
use core::time::Duration;
use ockam_core::compat::{sync::Arc, vec::Vec};
use ockam_core::flow_control::FlowControlPolicy;
use ockam_core::{
    errcode::{Kind, Origin},
    route, Address, AllowAll, AllowOnwardAddress, Error, LocalMessage, Mailboxes, Message,
    RelayMessage, Result, Route, Routed, TransportMessage,
};
use ockam_core::{LocalInfo, Mailbox};

/// Full set of options to `send_and_receive_extended` function
pub struct MessageSendReceiveOptions {
    message_wait: MessageWait,
}

impl Default for MessageSendReceiveOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageSendReceiveOptions {
    /// Default options with [`DEFAULT_TIMEOUT`] and no flow control
    pub fn new() -> Self {
        Self {
            message_wait: MessageWait::Timeout(Duration::from_secs(DEFAULT_TIMEOUT)),
        }
    }

    /// Set custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.message_wait = MessageWait::Timeout(timeout);
        self
    }

    /// Wait for the message forever
    pub fn without_timeout(mut self) -> Self {
        self.message_wait = MessageWait::Blocking;
        self
    }
}

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
    pub async fn send_and_receive<M>(&self, route: impl Into<Route>, msg: impl Message) -> Result<M>
    where
        M: Message,
    {
        Ok(self
            .send_and_receive_extended::<M>(route, msg, MessageSendReceiveOptions::new())
            .await?
            .body())
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
    pub async fn send_and_receive_extended<M>(
        &self,
        route: impl Into<Route>,
        msg: impl Message,
        options: MessageSendReceiveOptions,
    ) -> Result<Routed<M>>
    where
        M: Message,
    {
        let route: Route = route.into();

        let next = route.next()?.clone();
        let address = Address::random_tagged("Context.send_and_receive.detached");
        let mailboxes = Mailboxes::new(
            Mailbox::new(
                address.clone(),
                Arc::new(AllowAll),
                Arc::new(AllowOnwardAddress(next.clone())),
            ),
            vec![],
        );

        if let Some(flow_control_id) = self
            .flow_controls
            .find_flow_control_with_producer_address(&next)
            .map(|x| x.flow_control_id().clone())
        {
            // To be able to receive the response
            self.flow_controls.add_consumer(
                address,
                &flow_control_id,
                FlowControlPolicy::ProducerAllowMultiple,
            );
        }

        let mut child_ctx = self.new_detached_with_mailboxes(mailboxes).await?;

        child_ctx.send(route, msg).await?;
        child_ctx
            .receive_extended::<M>(
                MessageReceiveOptions::new().with_message_wait(options.message_wait),
            )
            .await
    }

    /// Send a message to another address associated with this worker
    ///
    /// This function is a simple wrapper around `Self::send()` which
    /// validates the address given to it and will reject invalid
    /// addresses.
    pub async fn send_to_self<A, M>(&self, from: A, addr: A, msg: M) -> Result<()>
    where
        A: Into<Address>,
        M: Message + Send + 'static,
    {
        let addr = addr.into();
        if self.mailboxes.contains(&addr) {
            self.send_from_address(addr, msg, from.into()).await
        } else {
            Err(NodeError::NodeState(NodeReason::Unknown).internal())
        }
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
    /// # use {ockam_node::Context, ockam_core::Result};
    /// # async fn test(ctx: &mut Context) -> Result<()> {
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
    /// ctx.send("my-test-worker", MyMessage::new("Hello you there :)")).await?;
    /// Ok(())
    /// # }
    /// ```
    pub async fn send<R, M>(&self, route: R, msg: M) -> Result<()>
    where
        R: Into<Route>,
        M: Message + Send + 'static,
    {
        self.send_from_address(route.into(), msg, self.address())
            .await
    }

    /// Send a message to an address or via a fully-qualified route
    /// after attaching the given [`LocalInfo`] to the message.
    pub async fn send_with_local_info<R, M>(
        &self,
        route: R,
        msg: M,
        local_info: Vec<LocalInfo>,
    ) -> Result<()>
    where
        R: Into<Route>,
        M: Message + Send + 'static,
    {
        self.send_from_address_impl(route.into(), msg, self.address(), local_info)
            .await
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
        M: Message + Send + 'static,
    {
        self.send_from_address_impl(route.into(), msg, sending_address, Vec::new())
            .await
    }

    async fn send_from_address_impl<M>(
        &self,
        route: Route,
        msg: M,
        sending_address: Address,
        local_info: Vec<LocalInfo>,
    ) -> Result<()>
    where
        M: Message + Send + 'static,
    {
        // Check if the sender address exists
        if !self.mailboxes.contains(&sending_address) {
            return Err(Error::new_without_cause(Origin::Node, Kind::Invalid));
        }

        // First resolve the next hop in the route
        let (reply_tx, mut reply_rx) = small_channel();
        let next = match route.next() {
            Ok(next) => next,
            Err(err) => {
                // TODO: communicate bad routes to calling function
                tracing::error!("Invalid route for message sent from {}", sending_address);
                return Err(err);
            }
        };

        let req = NodeMessage::SenderReq(next.clone(), reply_tx);
        self.sender
            .send(req)
            .await
            .map_err(NodeError::from_send_err)?;
        let (addr, sender) = reply_rx
            .recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??
            .take_sender()?;

        // Pack the payload into a TransportMessage
        let payload = msg.encode().map_err(|_| NodeError::Data.internal())?;
        let transport_msg = TransportMessage::v1(route, route![sending_address.clone()], payload);

        // Pack transport message into a LocalMessage wrapper
        let local_msg = LocalMessage::new(transport_msg, local_info);

        // Pack local message into a RelayMessage wrapper
        let relay_msg = RelayMessage::new(sending_address.clone(), addr, local_msg);

        debugger::log_outgoing_message(self, &relay_msg);

        if !self.mailboxes.is_outgoing_authorized(&relay_msg).await? {
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

    /// Forward a transport message to its next routing destination
    ///
    /// Similar to [`Context::send`], but taking a
    /// [`TransportMessage`], which contains the full destination
    /// route, and calculated return route for this hop.
    ///
    /// **Note:** you most likely want to use
    /// [`Context::send`] instead, unless you are writing an
    /// external router implementation for ockam node.
    ///
    /// [`Context::send`]: crate::Context::send
    /// [`TransportMessage`]: ockam_core::TransportMessage
    pub async fn forward(&self, local_msg: LocalMessage) -> Result<()> {
        self.forward_from_address(local_msg, self.address()).await
    }

    /// Forward a transport message to its next routing destination
    ///
    /// Similar to [`Context::send`], but taking a
    /// [`TransportMessage`], which contains the full destination
    /// route, and calculated return route for this hop.
    ///
    /// **Note:** you most likely want to use
    /// [`Context::send`] instead, unless you are writing an
    /// external router implementation for ockam node.
    ///
    /// [`Context::send`]: crate::Context::send
    /// [`TransportMessage`]: ockam_core::TransportMessage
    pub async fn forward_from_address(
        &self,
        local_msg: LocalMessage,
        sending_address: Address,
    ) -> Result<()> {
        // Check if the sender address exists
        if !self.mailboxes.contains(&sending_address) {
            return Err(Error::new_without_cause(Origin::Node, Kind::Invalid));
        }

        // First resolve the next hop in the route
        let (reply_tx, mut reply_rx) = small_channel();
        let next = match local_msg.transport().onward_route.next() {
            Ok(next) => next,
            Err(_) => {
                // TODO: communicate bad routes to calling function
                tracing::error!(
                    "Invalid onward route for message forwarded from {}",
                    local_msg.transport().return_route
                );
                panic!("invalid destination route");
            }
        };
        let req = NodeMessage::SenderReq(next.clone(), reply_tx);
        self.sender
            .send(req)
            .await
            .map_err(NodeError::from_send_err)?;
        let (addr, sender) = reply_rx
            .recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??
            .take_sender()?;

        // Pack the transport message into a RelayMessage wrapper
        let relay_msg = RelayMessage::new(sending_address, addr, local_msg);

        debugger::log_outgoing_message(self, &relay_msg);

        // TODO check if this context is allowed to forward the message
        //      to the next hop in the route
        if !self.mailboxes.is_outgoing_authorized(&relay_msg).await? {
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
}
