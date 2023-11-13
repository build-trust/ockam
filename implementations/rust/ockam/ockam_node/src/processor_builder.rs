use crate::error::{NodeError, NodeReason};
use crate::{debugger, AddressMetadata};
use crate::{relay::ProcessorRelay, Context, NodeMessage};
use alloc::string::String;
use ockam_core::compat::{sync::Arc, vec::Vec};
use ockam_core::{
    errcode::{Kind, Origin},
    Address, DenyAll, Error, IncomingAccessControl, Mailboxes, OutgoingAccessControl, Processor,
    Result,
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
    /// Worker with only one [`Address`]
    pub fn with_address(self, address: impl Into<Address>) -> ProcessorBuilderOneAddress<P> {
        ProcessorBuilderOneAddress {
            incoming_ac: Arc::new(DenyAll),
            outgoing_ac: Arc::new(DenyAll),
            processor: self.processor,
            address: address.into(),
            metadata: None,
        }
    }

    /// Worker with multiple [`Address`]es
    pub fn with_mailboxes(self, mailboxes: Mailboxes) -> ProcessorBuilderMultipleAddresses<P> {
        ProcessorBuilderMultipleAddresses {
            mailboxes,
            processor: self.processor,
            metadata_list: vec![],
        }
    }
}

pub struct ProcessorBuilderMultipleAddresses<P>
where
    P: Processor<Context = Context>,
{
    mailboxes: Mailboxes,
    processor: P,
    metadata_list: Vec<AddressMetadata>,
}

impl<P> ProcessorBuilderMultipleAddresses<P>
where
    P: Processor<Context = Context>,
{
    /// Mark the provided address as terminal
    pub fn terminal(mut self, address: impl Into<Address>) -> Self {
        let address = address.into();
        let metadata = self.metadata_list.iter_mut().find(|m| m.address == address);

        if let Some(metadata) = metadata {
            metadata.is_terminal = true;
        } else {
            self.metadata_list.push(AddressMetadata {
                address,
                is_terminal: true,
                attributes: vec![],
            });
        }
        self
    }

    /// Adds metadata attribute for the provided address
    pub fn with_metadata_attribute(
        mut self,
        address: impl Into<Address>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let address = address.into();
        let metadata = self.metadata_list.iter_mut().find(|m| m.address == address);

        if let Some(metadata) = metadata {
            metadata.attributes.push((key.into(), value.into()));
        } else {
            self.metadata_list.push(AddressMetadata {
                address,
                is_terminal: false,
                attributes: vec![(key.into(), value.into())],
            });
        }
        self
    }

    /// Consume this builder and start a new Ockam [`Processor`] from the given context
    pub async fn start(self, context: &Context) -> Result<()> {
        start(context, self.mailboxes, self.processor, self.metadata_list).await
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
}

impl<P> ProcessorBuilderOneAddress<P>
where
    P: Processor<Context = Context>,
{
    /// Mark the address as terminal
    pub fn terminal(mut self) -> Self {
        if let Some(metadata) = self.metadata.as_mut() {
            metadata.is_terminal = true;
        } else {
            self.metadata = Some(AddressMetadata {
                address: self.address.clone(),
                is_terminal: true,
                attributes: vec![],
            });
        }
        self
    }

    /// Adds metadata attribute
    pub fn with_metadata_attribute(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        if let Some(metadata) = self.metadata.as_mut() {
            metadata.attributes.push((key.into(), value.into()));
        } else {
            self.metadata = Some(AddressMetadata {
                address: self.address.clone(),
                is_terminal: false,
                attributes: vec![(key.into(), value.into())],
            });
        }
        self
    }

    /// Consume this builder and start a new Ockam [`Processor`] from the given context
    pub async fn start(self, context: &Context) -> Result<()> {
        start(
            context,
            Mailboxes::main(self.address, self.incoming_ac, self.outgoing_ac),
            self.processor,
            self.metadata.map(|m| vec![m]).unwrap_or_else(Vec::new),
        )
        .await
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
}

/// Consume this builder and start a new Ockam [`Processor`] from the given context

pub async fn start<P>(
    context: &Context,
    mailboxes: Mailboxes,
    processor: P,
    metadata: Vec<AddressMetadata>,
) -> Result<()>
where
    P: Processor<Context = Context>,
{
    info!(
        "Initializing ockam processor '{}' with access control in:{:?} out:{:?}",
        mailboxes.main_address(),
        mailboxes.main_mailbox().incoming_access_control(),
        mailboxes.main_mailbox().outgoing_access_control(),
    );

    let addresses = mailboxes.addresses();

    // Pass it to the context
    let (ctx, sender, ctrl_rx) = context.copy_with_mailboxes(mailboxes);

    debugger::log_inherit_context("PROCESSOR", context, &ctx);

    // Then initialise the processor message relay
    ProcessorRelay::<P>::init(context.runtime(), processor, ctx, ctrl_rx);

    // Send start request to router
    let (msg, mut rx) = NodeMessage::start_processor(addresses, sender, metadata);
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
