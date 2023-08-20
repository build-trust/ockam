use crate::debugger;
use crate::error::{NodeError, NodeReason};
use crate::{relay::WorkerRelay, Context, NodeMessage};
use ockam_core::compat::sync::Arc;
use ockam_core::{
    errcode::{Kind, Origin},
    Address, AllowAll, Error, IncomingAccessControl, Mailboxes, OutgoingAccessControl, Result,
    Worker,
};

/// Start a [`Worker`] with a custom configuration
///
/// Varying use-cases should use the builder API to customise the
/// underlying worker that is created.
pub struct WorkerBuilder<W>
where
    W: Worker<Context = Context>,
{
    worker: W,
}

impl<W> WorkerBuilder<W>
where
    W: Worker<Context = Context>,
{
    /// Create a new builder for a given Worker. Default AccessControl is AllowAll
    pub fn new(worker: W) -> Self {
        Self { worker }
    }
}

impl<W> WorkerBuilder<W>
where
    W: Worker<Context = Context>,
{
    /// Worker with only one [`Address`]
    pub fn with_address(self, address: impl Into<Address>) -> WorkerBuilderOneAddress<W> {
        WorkerBuilderOneAddress {
            incoming_ac: Arc::new(AllowAll),
            outgoing_ac: Arc::new(AllowAll),
            worker: self.worker,
            address: address.into(),
        }
    }

    /// Worker with multiple [`Address`]es
    pub fn with_mailboxes(self, mailboxes: Mailboxes) -> WorkerBuilderMultipleAddresses<W> {
        WorkerBuilderMultipleAddresses {
            mailboxes,
            worker: self.worker,
        }
    }
}

pub struct WorkerBuilderMultipleAddresses<W>
where
    W: Worker<Context = Context>,
{
    mailboxes: Mailboxes,
    worker: W,
}

impl<W> WorkerBuilderMultipleAddresses<W>
where
    W: Worker<Context = Context>,
{
    /// Consume this builder and start a new Ockam [`Worker`] from the given context
    pub async fn start(self, context: &Context) -> Result<()> {
        start(context, self.mailboxes, self.worker).await
    }
}

pub struct WorkerBuilderOneAddress<W>
where
    W: Worker<Context = Context>,
{
    incoming_ac: Arc<dyn IncomingAccessControl>,
    outgoing_ac: Arc<dyn OutgoingAccessControl>,
    address: Address,
    worker: W,
}

impl<W> WorkerBuilderOneAddress<W>
where
    W: Worker<Context = Context>,
{
    /// Consume this builder and start a new Ockam [`Worker`] from the given context
    pub async fn start(self, context: &Context) -> Result<()> {
        start(
            context,
            Mailboxes::main(self.address, self.incoming_ac, self.outgoing_ac),
            self.worker,
        )
        .await
    }
}

impl<W> WorkerBuilderOneAddress<W>
where
    W: Worker<Context = Context>,
{
    /// Set [`IncomingAccessControl`]
    pub fn with_incoming_access_control(
        mut self,
        incoming_access_control: impl IncomingAccessControl,
    ) -> Self {
        self.incoming_ac = Arc::new(incoming_access_control);
        self
    }

    /// Set [`IncomingAccessControl`]
    pub fn with_incoming_access_control_arc(
        mut self,
        incoming_access_control: Arc<dyn IncomingAccessControl>,
    ) -> Self {
        self.incoming_ac = incoming_access_control.clone();
        self
    }

    /// Set [`OutgoingAccessControl`]
    pub fn with_outgoing_access_control(
        mut self,
        outgoing_access_control: impl OutgoingAccessControl,
    ) -> Self {
        self.outgoing_ac = Arc::new(outgoing_access_control);
        self
    }

    /// Set [`OutgoingAccessControl`]
    pub fn with_outgoing_access_control_arc(
        mut self,
        outgoing_access_control: Arc<dyn OutgoingAccessControl>,
    ) -> Self {
        self.outgoing_ac = outgoing_access_control.clone();
        self
    }
}

/// Consume this builder and start a new Ockam [`Worker`] from the given context
async fn start<W>(context: &Context, mailboxes: Mailboxes, worker: W) -> Result<()>
where
    W: Worker<Context = Context>,
{
    debug!(
        "Initializing ockam worker '{}' with access control in:{:?} out:{:?}",
        mailboxes.main_address(),
        mailboxes.main_mailbox().incoming_access_control(),
        mailboxes.main_mailbox().outgoing_access_control(),
    );

    let addresses = mailboxes.addresses();

    // Pass it to the context
    let (ctx, sender, ctrl_rx) = context.copy_with_mailboxes(mailboxes);

    debugger::log_inherit_context("WORKER", context, &ctx);

    // Then initialise the worker message relay
    WorkerRelay::init(context.runtime(), worker, ctx, ctrl_rx);

    // Send start request to router
    let (msg, mut rx) =
        NodeMessage::start_worker(addresses, sender, false, context.mailbox_count());
    context
        .sender()
        .send(msg)
        .await
        .map_err(|e| Error::new(Origin::Node, Kind::Invalid, e))?;

    // Wait for the actual return code
    rx.recv()
        .await
        .ok_or_else(|| NodeError::NodeState(NodeReason::Unknown).internal())??;

    Ok(())
}
