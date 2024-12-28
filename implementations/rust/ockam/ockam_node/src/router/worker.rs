use crate::router::record::{AddressRecord, WorkerMeta};
use crate::router::{Router, RouterState, SenderPair};
use crate::WorkerShutdownPriority;
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
        shutdown_priority: WorkerShutdownPriority,
        metrics: Arc<AtomicUsize>,
    ) -> Result<()> {
        if *self.state.read().unwrap() != RouterState::Running {
            return Err(Error::new(
                Origin::Node,
                Kind::Shutdown,
                "The node is shutting down",
            ))?;
        }

        self.add_worker_impl(mailboxes, senders, detached, shutdown_priority, metrics)
    }

    fn add_worker_impl(
        &self,
        mailboxes: &Mailboxes,
        senders: SenderPair,
        detached: bool,
        shutdown_priority: WorkerShutdownPriority,
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
            WorkerMeta {
                processor: false,
                detached,
            },
            shutdown_priority,
            metrics,
        );

        self.map.insert_address_record(address_record, mailboxes)?;

        Ok(())
    }
}
