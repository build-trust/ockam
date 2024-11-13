use crate::{debugger, ContextMode, WorkerShutdownPriority};
use crate::{relay::ProcessorRelay, Context};
use ockam_core::compat::string::String;
use ockam_core::compat::sync::Arc;
use ockam_core::{
    Address, AddressMetadata, DenyAll, IncomingAccessControl, Mailbox, Mailboxes,
    OutgoingAccessControl, Processor, Result,
};

/// Start a [`Processor`]
///
/// Varying use-cases should use the builder API to customise the
/// underlying processor that is created.
pub struct ProcessorBuilder<P>
where
    P: Processor<Context = Context>,
{
    processor: P,
}

impl<P> ProcessorBuilder<P>
where
    P: Processor<Context = Context>,
{
    /// Create a new builder for a given Processor. Default AccessControl is DenyAll
    pub fn new(processor: P) -> Self {
        Self { processor }
    }
}

impl<P> ProcessorBuilder<P>
where
    P: Processor<Context = Context>,
{
    /// Processor with only one [`Address`]
    pub fn with_address(self, address: impl Into<Address>) -> ProcessorBuilderOneAddress<P> {
        self.with_address_and_metadata_impl(address, None)
    }

    /// Processor with single terminal [`Address`]
    pub fn with_terminal_address(
        self,
        address: impl Into<Address>,
    ) -> ProcessorBuilderOneAddress<P> {
        self.with_address_and_metadata(
            address,
            AddressMetadata {
                is_terminal: true,
                attributes: vec![],
            },
        )
    }

    /// Processor with single terminal [`Address`] and metadata
    pub fn with_address_and_metadata(
        self,
        address: impl Into<Address>,
        metadata: AddressMetadata,
    ) -> ProcessorBuilderOneAddress<P> {
        self.with_address_and_metadata_impl(address, Some(metadata))
    }

    /// Processor with single terminal [`Address`] and metadata
    pub fn with_address_and_metadata_impl(
        self,
        address: impl Into<Address>,
        metadata: Option<AddressMetadata>,
    ) -> ProcessorBuilderOneAddress<P> {
        ProcessorBuilderOneAddress {
            incoming_ac: Arc::new(DenyAll),
            outgoing_ac: Arc::new(DenyAll),
            processor: self.processor,
            address: address.into(),
            metadata,
            shutdown_priority: Default::default(),
        }
    }

    /// Worker with multiple [`Address`]es
    pub fn with_mailboxes(self, mailboxes: Mailboxes) -> ProcessorBuilderMultipleAddresses<P> {
        ProcessorBuilderMultipleAddresses {
            mailboxes,
            shutdown_priority: Default::default(),
            processor: self.processor,
        }
    }
}

pub struct ProcessorBuilderMultipleAddresses<P>
where
    P: Processor<Context = Context>,
{
    mailboxes: Mailboxes,
    shutdown_priority: WorkerShutdownPriority,
    processor: P,
}

impl<P> ProcessorBuilderMultipleAddresses<P>
where
    P: Processor<Context = Context>,
{
    /// Consume this builder and start a new Ockam [`Processor`] from the given context
    pub fn start(self, context: &Context) -> Result<()> {
        start(
            context,
            self.mailboxes,
            self.shutdown_priority,
            self.processor,
        )
    }

    pub fn with_shutdown_priority(mut self, shutdown_priority: WorkerShutdownPriority) -> Self {
        self.shutdown_priority = shutdown_priority;
        self
    }
}

pub struct ProcessorBuilderOneAddress<P>
where
    P: Processor<Context = Context>,
{
    incoming_ac: Arc<dyn IncomingAccessControl>,
    outgoing_ac: Arc<dyn OutgoingAccessControl>,
    address: Address,
    processor: P,
    metadata: Option<AddressMetadata>,
    shutdown_priority: WorkerShutdownPriority,
}

impl<P> ProcessorBuilderOneAddress<P>
where
    P: Processor<Context = Context>,
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

    /// Consume this builder and start a new Ockam [`Processor`] from the given context
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
            self.processor,
        )
    }
}

impl<P> ProcessorBuilderOneAddress<P>
where
    P: Processor<Context = Context>,
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

    pub fn with_shutdown_priority(mut self, shutdown_priority: WorkerShutdownPriority) -> Self {
        self.shutdown_priority = shutdown_priority;
        self
    }
}

/// Consume this builder and start a new Ockam [`Processor`] from the given context
pub fn start<P>(
    context: &Context,
    mailboxes: Mailboxes,
    shutdown_priority: WorkerShutdownPriority,
    processor: P,
) -> Result<()>
where
    P: Processor<Context = Context>,
{
    debug!(
        "Initializing ockam processor '{}' with access control in:{:?} out:{:?}",
        mailboxes.primary_address(),
        mailboxes.primary_mailbox().incoming_access_control(),
        mailboxes.primary_mailbox().outgoing_access_control(),
    );

    // Pass it to the context
    let (ctx, sender, ctrl_rx) = context.new_with_mailboxes(mailboxes, ContextMode::Attached);

    debugger::log_inherit_context("PROCESSOR", context, &ctx);

    let router = context.router()?;
    router.add_processor(ctx.mailboxes(), sender, shutdown_priority)?;

    // Then initialise the processor message relay
    ProcessorRelay::<P>::init(context.runtime(), processor, ctx, ctrl_rx);

    Ok(())
}
