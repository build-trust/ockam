use crate::debugger;
use crate::error::{NodeError, NodeReason};
use crate::{relay::ProcessorRelay, Context, NodeMessage};
use ockam_core::compat::sync::Arc;
use ockam_core::{
    errcode::{Kind, Origin},
    AccessControl, Address, AllowAll, Error, Mailboxes, Processor, Result,
};

/// Start a [`Processor`] with a custom [`AccessControl`] configuration
///
/// Any incoming messages for the processor will first be subject to the
/// configured `AccessControl` before it is passed on to
/// [`Processor::handle_message`].
///
/// The [`Context::start_processor()`] function wraps this type and
/// simply calls `ProcessorBuilder::with_inherited_access_control()`.
///
/// Varying use-cases should use the builder API to customise the
/// underlying processor that is created.
pub struct ProcessorBuilder<P> {
    mailboxes: Mailboxes,
    processor: P,
}

impl<P> ProcessorBuilder<P>
where
    P: Processor<Context = Context>,
{
    /// Create a processor with `AllowAll` access control
    pub fn without_access_control(address: impl Into<Address>, processor: P) -> Self {
        let mailboxes = Mailboxes::main(address.into(), Arc::new(AllowAll), Arc::new(AllowAll));
        // TODO: @ac can we just default DenyAll ?
        //let mailboxes = Mailboxes::from_address_set(address.into(), Arc::new(ockam_core::DenyAll), Arc::new(ockam_core::DenyAll));

        Self {
            mailboxes,
            processor,
        }
    }

    /// Create a processor which inherits access control from the given context
    pub fn with_inherited_access_control(
        context: &Context,
        address: impl Into<Address>,
        processor: P,
    ) -> Self {
        let address = address.into();

        // Inherit access control from the given context's main mailbox
        let incoming_access_control = context
            .mailboxes()
            .main_mailbox()
            .incoming_access_control()
            .clone();
        let outgoing_access_control = context
            .mailboxes()
            .main_mailbox()
            .outgoing_access_control()
            .clone();
        // TODO: @ac can we just default DenyAll ?
        // let access_control = Arc::new(ockam_core::DenyAll);

        debug!(
            "Processor '{}' inherits access control incoming: '{:?}' outgoing: '{:?}' from: '{}'",
            address,
            incoming_access_control,
            outgoing_access_control,
            context.address(),
        );

        let mailboxes = Mailboxes::main(address, incoming_access_control, outgoing_access_control);

        Self {
            mailboxes,
            processor,
        }
    }

    /// Create a processor which uses the given access control
    pub fn with_access_control<AC>(
        incoming_access_control: Arc<dyn AccessControl>,
        outgoing_access_control: Arc<dyn AccessControl>,
        address: impl Into<Address>,
        processor: P,
    ) -> Self {
        // TODO: @ac can we just default DenyAll ?
        //let access_control = ockam_core::DenyAll;
        let mailboxes = Mailboxes::main(
            address.into(),
            incoming_access_control,
            outgoing_access_control,
        );

        Self {
            mailboxes,
            processor,
        }
    }

    /// Create a processor which uses the access control from the given
    /// [`Mailboxes`]
    pub fn with_mailboxes(mailboxes: Mailboxes, processor: P) -> Self {
        Self {
            mailboxes,
            processor,
        }
    }

    /// Consume this builder and start a new Ockam [`Processor`] from the given context
    #[inline]
    pub async fn start(self, context: &Context) -> Result<Address> {
        info!(
            "Initializing ockam processor '{}' with access control in:{:?} out:{:?}",
            self.mailboxes.main_address(),
            self.mailboxes.main_mailbox().incoming_access_control(),
            self.mailboxes.main_mailbox().outgoing_access_control(),
        );

        let mailboxes = self.mailboxes;
        let main_address = mailboxes.main_address().clone();

        // Pass it to the context
        let (ctx, sender, ctrl_rx) = Context::new(
            context.runtime().clone(),
            context.sender().clone(),
            mailboxes,
            None,
        );

        debugger::log_inherit_context("PROCESSOR", context, &ctx);

        // Then initialise the processor message relay
        ProcessorRelay::<P>::init(context.runtime(), self.processor, ctx, ctrl_rx);

        // Send start request to router
        let (msg, mut rx) = NodeMessage::start_processor(main_address.clone(), sender);
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
