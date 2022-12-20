use crate::debugger;
use crate::error::{NodeError, NodeReason};
use crate::{relay::WorkerRelay, Context, NodeMessage};
use ockam_core::compat::sync::Arc;
use ockam_core::{
    errcode::{Kind, Origin},
    AccessControl, Address, Error, Mailboxes, Message, Result, Worker,
};

/// Start a [`Worker`] with a custom [`AccessControl`] configuration
///
/// Any incoming messages for the worker will first be subject to the
/// configured `AccessControl` before it is passed on to
/// [`Worker::handle_message`].
///
/// The [`Context::start_worker()`] function wraps this type and
/// simply calls `WorkerBuilder::with_inherited_access_control()`.
///
/// Varying use-cases should use the builder API to customise the
/// underlying worker that is created.
pub struct WorkerBuilder<W> {
    mailboxes: Mailboxes,
    worker: W,
}

impl<M, W> WorkerBuilder<W>
where
    M: Message + Send + 'static,
    W: Worker<Context = Context, Message = M>,
{
    /// Create a worker which uses the given access control
    pub fn with_access_control(
        incoming_access_control: Arc<dyn AccessControl>,
        outgoing_access_control: Arc<dyn AccessControl>,
        address: impl Into<Address>,
        worker: W,
    ) -> Self {
        let mailboxes = Mailboxes::main(
            address.into(),
            incoming_access_control,
            outgoing_access_control,
        );

        Self { mailboxes, worker }
    }

    /// Create a worker which uses the access control from the given
    /// [`Mailboxes`]
    pub fn with_mailboxes(mailboxes: Mailboxes, worker: W) -> Self {
        Self { mailboxes, worker }
    }

    /// Consume this builder and start a new Ockam [`Worker`] from the given context
    #[inline]
    pub async fn start(self, context: &Context) -> Result<Address> {
        info!(
            "Initializing ockam worker '{}' with access control in:{:?} out:{:?}",
            self.mailboxes.main_address(),
            self.mailboxes.main_mailbox().incoming_access_control(),
            self.mailboxes.main_mailbox().outgoing_access_control(),
        );

        let mailboxes = self.mailboxes;
        let addresses = mailboxes.addresses();
        let main_address = mailboxes.main_address().clone();

        // Pass it to the context
        let (ctx, sender, ctrl_rx) = Context::new(
            context.runtime().clone(),
            context.sender().clone(),
            mailboxes,
            None,
        );

        debugger::log_inherit_context("WORKER", context, &ctx);

        // Then initialise the worker message relay
        WorkerRelay::<W, M>::init(context.runtime(), self.worker, ctx, ctrl_rx);

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

        Ok(main_address)
    }
}
