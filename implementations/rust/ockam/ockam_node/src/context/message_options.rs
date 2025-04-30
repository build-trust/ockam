use crate::DEFAULT_TIMEOUT;

use core::time::Duration;
use ockam_core::compat::sync::Arc;
use ockam_core::compat::vec::Vec;
use ockam_core::{Address, IncomingAccessControl, LocalInfo, OutgoingAccessControl};

/// Full set of options to `send_and_receive_extended` function
#[derive(Clone, Debug)]
pub struct MessageSendReceiveOptions {
    send: MessageSendOptions,
    receive: MessageReceiveOptions,
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
            receive: MessageReceiveOptions::new(),
            send: MessageSendOptions::new(),
        }
    }

    /// Set custom timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.receive = self.receive.with_timeout(timeout);
        self
    }

    /// Wait for the message forever
    pub fn without_timeout(mut self) -> Self {
        self.receive = self.receive.without_timeout();
        self
    }

    /// Set incoming access control
    pub fn with_incoming_access_control(
        mut self,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
    ) -> Self {
        self.receive = self
            .receive
            .with_incoming_access_control(incoming_access_control);
        self
    }

    /// Set outgoing access control
    pub fn with_outgoing_access_control(
        mut self,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Self {
        self.send = self
            .send
            .with_outgoing_access_control(outgoing_access_control);
        self
    }

    /// Receive options
    pub fn receive(&self) -> &MessageReceiveOptions {
        &self.receive
    }

    /// Send options
    pub fn send(&self) -> &MessageSendOptions {
        &self.send
    }

    /// Consume self and return fields
    pub fn dissolve(self) -> (MessageSendOptions, MessageReceiveOptions) {
        (self.send, self.receive)
    }
}

/// Set of options to send a message
#[derive(Clone, Debug)]
pub struct MessageSendOptions {
    sending_address: Option<Address>,
    local_info: Vec<LocalInfo>,
    outgoing_access_control: Option<Arc<dyn OutgoingAccessControl>>,
}

impl Default for MessageSendOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageSendOptions {
    /// Default options
    pub fn new() -> Self {
        Self {
            sending_address: None,
            local_info: vec![],
            outgoing_access_control: None,
        }
    }

    /// Set LocalInfo
    pub fn with_local_info(mut self, local_info: Vec<LocalInfo>) -> Self {
        self.local_info = local_info;
        self
    }

    /// Set sending address (otherwise primary address is used)
    pub fn with_sending_address(mut self, sending_address: Address) -> Self {
        self.sending_address = Some(sending_address);
        self
    }

    /// Outgoing access control
    /// NOTE: Takes precedence over Context own outgoing Access Control
    pub fn with_outgoing_access_control(
        mut self,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Self {
        self.outgoing_access_control = Some(outgoing_access_control);
        self
    }

    /// Outgoing access control (for the request).
    /// NOTE: Takes precedence over Context own outgoing Access Control
    pub fn outgoing_access_control(&self) -> &Option<Arc<dyn OutgoingAccessControl>> {
        &self.outgoing_access_control
    }

    /// LocalInfo
    pub fn local_info(&self) -> &[LocalInfo] {
        &self.local_info
    }

    /// Consume self and return fields
    pub fn dissolve(
        self,
    ) -> (
        Option<Address>,
        Vec<LocalInfo>,
        Option<Arc<dyn OutgoingAccessControl>>,
    ) {
        (
            self.sending_address,
            self.local_info,
            self.outgoing_access_control,
        )
    }
}

/// Wait for a message
#[derive(Clone, Copy, Debug)]
pub enum MessageWait {
    /// Timeout
    Timeout(Duration),
    /// Blocking
    Blocking,
}

/// Set of options to receive a message
#[derive(Clone, Debug)]
pub struct MessageReceiveOptions {
    message_wait: MessageWait,
    incoming_access_control: Option<Arc<dyn IncomingAccessControl>>,
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
            incoming_access_control: None,
        }
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

    /// Incoming access control (for the response).
    /// NOTE: Takes precedence over Context own incoming Access Control
    pub fn with_incoming_access_control(
        mut self,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
    ) -> Self {
        self.incoming_access_control = Some(incoming_access_control);
        self
    }

    /// Wait for response
    pub fn message_wait(&self) -> &MessageWait {
        &self.message_wait
    }

    /// Incoming access control (for the response).
    /// NOTE: Takes precedence over Context own incoming Access Control
    pub fn incoming_access_control(&self) -> &Option<Arc<dyn IncomingAccessControl>> {
        &self.incoming_access_control
    }

    /// Consume self and return fields
    pub fn dissolve(self) -> (MessageWait, Option<Arc<dyn IncomingAccessControl>>) {
        (self.message_wait, self.incoming_access_control)
    }
}
