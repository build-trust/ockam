use crate::{debugger, ContextMode, WorkerShutdownPriority};
use crate::{relay::WorkerRelay, Context};
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::{
    Address, AddressMetadata, AllowAll, IncomingAccessControl, Mailbox, Mailboxes,
    OutgoingAccessControl, Result, Worker,
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
        self.with_address_and_metadata_impl(address, None)
    }

    /// Worker with single terminal [`Address`]
    pub fn with_terminal_address(self, address: impl Into<Address>) -> WorkerBuilderOneAddress<W> {
        self.with_address_and_metadata(
            address,
            AddressMetadata {
                is_terminal: true,
                attributes: vec![],
            },
        )
    }

    /// Worker with single terminal [`Address`] and metadata
    pub fn with_address_and_metadata(
        self,
        address: impl Into<Address>,
        metadata: AddressMetadata,
    ) -> WorkerBuilderOneAddress<W> {
        self.with_address_and_metadata_impl(address, Some(metadata))
    }

    /// Worker with single terminal [`Address`] and metadata
    pub fn with_address_and_metadata_impl(
        self,
        address: impl Into<Address>,
        metadata: Option<AddressMetadata>,
    ) -> WorkerBuilderOneAddress<W> {
        WorkerBuilderOneAddress {
            incoming_ac: Arc::new(AllowAll),
            outgoing_ac: Arc::new(AllowAll),
            worker: self.worker,
            address: address.into(),
            metadata,
            shutdown_priority: Default::default(),
        }
    }

    /// Worker with multiple [`Address`]es
    pub fn with_mailboxes(self, mailboxes: Mailboxes) -> WorkerBuilderMultipleAddresses<W> {
        WorkerBuilderMultipleAddresses {
            mailboxes,
            shutdown_priority: Default::default(),
            worker: self.worker,
        }
    }
}

pub struct WorkerBuilderMultipleAddresses<W>
where
    W: Worker<Context = Context>,
{
    mailboxes: Mailboxes,
    shutdown_priority: WorkerShutdownPriority,
    worker: W,
}

impl<W> WorkerBuilderMultipleAddresses<W>
where
    W: Worker<Context = Context>,
{
    /// Consume this builder and start a new Ockam [`Worker`] from the given context
    pub fn start(self, context: &Context) -> Result<()> {
        start(context, self.mailboxes, self.shutdown_priority, self.worker)
    }

    pub fn with_shutdown_priority(mut self, shutdown_priority: WorkerShutdownPriority) -> Self {
        self.shutdown_priority = shutdown_priority;
        self
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
    metadata: Option<AddressMetadata>,
    shutdown_priority: WorkerShutdownPriority,
}

impl<W> WorkerBuilderOneAddress<W>
where
    W: Worker<Context = Context>,
{
    /// Mark the provided address as terminal
    pub fn terminal(mut self) -> Self {
        self.metadata
            .get_or_insert(AddressMetadata {
                is_terminal: false,
                attributes: vec![],
            })
            .is_terminal = true;
        self
    }

    /// Adds metadata attribute for the provided address
    pub fn with_metadata_attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.metadata
            .get_or_insert(AddressMetadata {
                is_terminal: false,
                attributes: vec![],
            })
            .attributes
            .push((key.into(), value.into()));

        self
    }

    pub fn with_shutdown_priority(mut self, shutdown_priority: WorkerShutdownPriority) -> Self {
        self.shutdown_priority = shutdown_priority;
        self
    }

    /// Consume this builder and start a new Ockam [`Worker`] from the given context
    pub fn start(self, context: &Context) -> Result<()> {
        start(
            context,
            Mailboxes::new(
                Mailbox::new(
                    self.address,
                    self.metadata,
                    self.incoming_ac,
                    self.outgoing_ac,
                ),
                vec![],
            ),
            self.shutdown_priority,
            self.worker,
        )
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
fn start<W>(
    context: &Context,
    mailboxes: Mailboxes,
    shutdown_priority: WorkerShutdownPriority,
    worker: W,
) -> Result<()>
where
    W: Worker<Context = Context>,
{
    debug!(
        "Initializing ockam worker '{}' with access control in:{:?} out:{:?}",
        mailboxes.primary_address(),
        mailboxes.primary_mailbox().incoming_access_control(),
        mailboxes.primary_mailbox().outgoing_access_control(),
    );

    // Pass it to the context
    let (ctx, sender, ctrl_rx) = context.new_with_mailboxes(mailboxes, ContextMode::Attached);

    debugger::log_inherit_context("WORKER", context, &ctx);

    let router = context.router()?;
    router.add_worker(
        ctx.mailboxes(),
        sender,
        false,
        shutdown_priority,
        context.mailbox_count(),
    )?;

    // Then initialise the worker message relay
    WorkerRelay::init(context.runtime(), worker, ctx, ctrl_rx);

    Ok(())
}
