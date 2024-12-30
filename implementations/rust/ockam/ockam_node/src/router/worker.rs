use crate::router::{AddressRecord, NodeState, Router, SenderPair, WorkerMeta};
use core::sync::atomic::AtomicUsize;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{compat::sync::Arc, Error, Mailboxes, Result};

impl Router {
    /// Start a new worker
    pub fn add_worker(
        &self,
        mailboxes: &Mailboxes,
        senders: SenderPair,
        detached: bool,
        metrics: Arc<AtomicUsize>,
    ) -> Result<()> {
        if !self.state.is_running() {
            match self.state.node_state() {
                NodeState::Stopping => Err(Error::new(
                    Origin::Node,
                    Kind::Shutdown,
                    "The node is shutting down",
                ))?,
                NodeState::Running => unreachable!(),
                NodeState::Stopped => unreachable!(),
            }
        } else {
            self.add_worker_impl(mailboxes, senders, detached, metrics)
        }
    }

    fn add_worker_impl(
        &self,
        mailboxes: &Mailboxes,
        senders: SenderPair,
        detached: bool,
        metrics: Arc<AtomicUsize>,
    ) -> Result<()> {
        debug!("Starting new worker '{}'", mailboxes.primary_address());
        let SenderPair { msgs, ctrl } = senders;

        // Create an address record and insert it into the internal map
        let address_record = AddressRecord::new(
            mailboxes.primary_address().clone(),
            mailboxes.additional_addresses().cloned().collect(),
            msgs,
            ctrl,
            metrics,
            WorkerMeta {
                processor: false,
                detached,
            },
        );

        self.map.insert_address_record(address_record, mailboxes)?;

        Ok(())
    }
}
