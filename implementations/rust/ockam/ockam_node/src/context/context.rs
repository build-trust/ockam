use crate::channel_types::{SmallReceiver, SmallSender};
use crate::tokio::runtime::Handle;
use crate::{error::*, AsyncDropSender, NodeMessage};
use core::sync::atomic::AtomicUsize;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::{Arc, RwLock};
use ockam_core::compat::{string::String, vec::Vec};
use ockam_core::flow_control::FlowControls;
use ockam_core::{async_trait, Address, Mailboxes, RelayMessage, Result, TransportType};

#[cfg(feature = "std")]
use core::fmt::{Debug, Formatter};
use ockam_transport_core::Transport;

/// A default timeout in seconds
pub const DEFAULT_TIMEOUT: u64 = 30;

/// Context contains Node state and references to the runtime.
pub struct Context {
    pub(super) mailboxes: Mailboxes,
    pub(super) sender: SmallSender<NodeMessage>,
    pub(super) rt: Handle,
    pub(super) receiver: SmallReceiver<RelayMessage>,
    pub(super) async_drop_sender: Option<AsyncDropSender>,
    pub(super) mailbox_count: Arc<AtomicUsize>,
    /// List of transports used to resolve external addresses to local workers in routes
    pub(super) transports: Arc<RwLock<HashMap<TransportType, Arc<dyn Transport>>>>,
    pub(super) flow_controls: FlowControls,
}

/// This trait can be used to integrate transports into a node
#[async_trait]
pub trait HasContext {
    /// Return a cloned context
    fn get_context(&self) -> &Context;
}

#[cfg(feature = "std")]
impl Debug for Context {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Context")
            .field("mailboxes", &self.mailboxes)
            .field("sender", &self.sender)
            .field("runtime", &self.rt)
            .finish()
    }
}

impl Context {
    /// Return runtime clone
    pub fn runtime(&self) -> &Handle {
        &self.rt
    }

    /// Return mailbox_count clone
    pub(crate) fn mailbox_count(&self) -> Arc<AtomicUsize> {
        self.mailbox_count.clone()
    }

    /// Return a reference to sender
    pub(crate) fn sender(&self) -> &SmallSender<NodeMessage> {
        &self.sender
    }

    /// Return the primary address of the current worker
    pub fn address(&self) -> Address {
        self.mailboxes.main_address()
    }

    /// Return all addresses of the current worker
    pub fn addresses(&self) -> Vec<Address> {
        self.mailboxes.addresses()
    }

    /// Return a reference to the mailboxes of this context
    pub fn mailboxes(&self) -> &Mailboxes {
        &self.mailboxes
    }

    /// Shared [`FlowControls`] instance
    pub fn flow_controls(&self) -> &FlowControls {
        &self.flow_controls
    }
}

impl Context {
    /// Assign the current worker to a cluster
    ///
    /// A cluster is a set of workers that should be stopped together
    /// when the node is stopped or parts of the system are reloaded.
    /// **This is not to be confused with supervisors!**
    ///
    /// By adding your worker to a cluster you signal to the runtime
    /// that your worker may be depended on by other workers that
    /// should be stopped first.
    ///
    /// **Your cluster name MUST NOT start with `_internals.` or
    /// `ockam.`!**
    ///
    /// Clusters are de-allocated in reverse order of their
    /// initialisation when the node is stopped.
    pub async fn set_cluster<S: Into<String>>(&self, label: S) -> Result<()> {
        let (msg, mut rx) = NodeMessage::set_cluster(self.address(), label.into());
        self.sender
            .send(msg)
            .await
            .map_err(NodeError::from_send_err)?;
        rx.recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??
            .is_ok()
    }

    /// Return a list of all available worker addresses on a node
    pub async fn list_workers(&self) -> Result<Vec<Address>> {
        let (msg, mut reply_rx) = NodeMessage::list_workers();

        self.sender
            .send(msg)
            .await
            .map_err(NodeError::from_send_err)?;

        reply_rx
            .recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??
            .take_workers()
    }

    /// Send a shutdown acknowledgement to the router
    pub(crate) async fn send_stop_ack(&self) -> Result<()> {
        self.sender
            .send(NodeMessage::StopAck(self.address()))
            .await
            .map_err(NodeError::from_send_err)?;
        Ok(())
    }

    /// This function is called by Relay to indicate a worker is initialised
    pub(crate) async fn set_ready(&mut self) -> Result<()> {
        self.sender
            .send(NodeMessage::set_ready(self.address()))
            .await
            .map_err(NodeError::from_send_err)?;
        Ok(())
    }

    /// Wait for a particular address to become "ready"
    pub async fn wait_for<A: Into<Address>>(&mut self, addr: A) -> Result<()> {
        let (msg, mut reply) = NodeMessage::get_ready(addr.into());
        self.sender
            .send(msg)
            .await
            .map_err(NodeError::from_send_err)?;

        // This call blocks until the address has become ready or is
        // dropped by the router
        reply
            .recv()
            .await
            .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??;
        Ok(())
    }
}
