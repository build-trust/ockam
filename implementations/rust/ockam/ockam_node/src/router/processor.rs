use super::{AddressRecord, NodeState, Router, SenderPair, WorkerMeta};
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Mailboxes, Result};

impl Router {
    /// Start a processor
    pub(crate) fn add_processor(&self, mailboxes: &Mailboxes, senders: SenderPair) -> Result<()> {
        if self.state.is_running() {
            self.add_processor_impl(mailboxes, senders)
        } else {
            match self.state.node_state() {
                NodeState::Stopping => Err(Error::new(
                    Origin::Node,
                    Kind::Shutdown,
                    "The node is shutting down",
                ))?,
                NodeState::Running => unreachable!(),
                NodeState::Stopped => unreachable!(),
            }
        }
    }

    fn add_processor_impl(&self, mailboxes: &Mailboxes, senders: SenderPair) -> Result<()> {
        debug!("Starting new processor '{}'", mailboxes.primary_address());
        let SenderPair { msgs, ctrl } = senders;

        let record = AddressRecord::new(
            mailboxes.primary_address().clone(),
            mailboxes.additional_addresses().cloned().collect(),
            msgs,
            ctrl,
            // We don't keep track of the mailbox count for processors
            // because, while they are able to send and receive messages
            // via their mailbox, most likely this metric is going to be
            // irrelevant.  We may want to re-visit this decision in the
            // future, if the way processors are used changes.
            Arc::new(0.into()),
            WorkerMeta {
                processor: true,
                detached: false,
            },
        );

        self.map.insert_address_record(record, mailboxes)?;

        Ok(())
    }
}
