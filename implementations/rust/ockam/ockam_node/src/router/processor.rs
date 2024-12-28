use super::{Router, RouterState, SenderPair};
use crate::router::record::{AddressRecord, WorkerMeta};
use crate::WorkerShutdownPriority;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Mailboxes, Result};

impl Router {
    /// Start a processor
    pub(crate) fn add_processor(
        &self,
        mailboxes: &Mailboxes,
        senders: SenderPair,
        shutdown_priority: WorkerShutdownPriority,
    ) -> Result<()> {
        if *self.state.read().unwrap() != RouterState::Running {
            return Err(Error::new(
                Origin::Node,
                Kind::Shutdown,
                "The node is shutting down",
            ))?;
        }

        self.add_processor_impl(mailboxes, senders, shutdown_priority)
    }

    fn add_processor_impl(
        &self,
        mailboxes: &Mailboxes,
        senders: SenderPair,
        shutdown_priority: WorkerShutdownPriority,
    ) -> Result<()> {
        debug!("Starting new processor '{}'", mailboxes.primary_address());
        let SenderPair { msgs, ctrl } = senders;

        let record = AddressRecord::new(
            mailboxes.primary_address().clone(),
            mailboxes.additional_addresses().cloned().collect(),
            msgs,
            ctrl,
            WorkerMeta {
                processor: true,
                detached: false,
            },
            shutdown_priority,
            // We don't keep track of the mailbox count for processors
            // because, while they are able to send and receive messages
            // via their mailbox, most likely this metric is going to be
            // irrelevant.  We may want to re-visit this decision in the
            // future, if the way processors are used changes.
            Arc::new(0.into()),
        );

        self.map.insert_address_record(record, mailboxes)?;

        Ok(())
    }
}
