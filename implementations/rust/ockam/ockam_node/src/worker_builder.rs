use crate::error::{NodeError, NodeReason};
use crate::{relay::WorkerRelay, Context, NodeMessage};
use ockam_core::compat::sync::Arc;
use ockam_core::{
    errcode::{Kind, Origin},
    AccessControl, Address, AddressSet, AllowAll, Error, Mailboxes, Message, Result, Worker,
};

/// Start a [`Worker`] with a custom [`AccessControl`] configuration
///
/// Any incoming messages for the worker will first be subject to the
/// configured `AccessControl` before it is passed on to
/// [`Worker::handle_message`].
///
/// The [`Context::start_worker()`] function wraps this type and
/// simply calls `WorkerBuilder::with_context_access_control()`.
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
    /// Create a worker with `AllowAll` access control
    pub fn without_access_control<AS>(address_set: AS, worker: W) -> Self
    where
        AS: Into<AddressSet>,
    {
        let mailboxes = Mailboxes::from_address_set(address_set.into(), Arc::new(AllowAll));

        Self { mailboxes, worker }
    }

    /// Create a worker which inherits access control from the given context
    pub fn with_inherited_access_control<AS>(context: &Context, address_set: AS, worker: W) -> Self
    where
        AS: Into<AddressSet>,
    {
        let address_set = address_set.into();

        // Inherit access control from the given context's main mailbox
        let access_control = context.mailboxes().main_mailbox().access_control().clone();

        debug!(
            "Worker '{}' inherits access control '{:?}' from: '{}'",
            address_set.first(),
            access_control,
            context.address(),
        );

        let mailboxes = Mailboxes::from_address_set(address_set, access_control);

        Self { mailboxes, worker }
    }

    /// Create a worker which uses the given access control
    pub fn with_access_control<A, AC>(access_control: AC, address: A, worker: W) -> Self
    where
        A: Into<Address>,
        AC: AccessControl,
    {
        let mailboxes = Mailboxes::main(address.into(), Arc::new(access_control));

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
            "Initializing ockam worker with access control: {:?}",
            self.mailboxes.main_mailbox().access_control(),
        );

        let mailboxes = self.mailboxes;
        let addresses = mailboxes.addresses();
        let main_address = mailboxes.main_address().clone();

        // Pass it to the context
        let (ctx, sender, ctrl_rx) =
            Context::new(context.runtime(), context.sender().clone(), mailboxes, None);

        // Then initialise the worker message relay
        WorkerRelay::<W, M>::init(&context.runtime(), self.worker, ctx, ctrl_rx);

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
