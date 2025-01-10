use crate::channel_types::{oneshot_channel, MessageSender, OneshotReceiver, OneshotSender};
use crate::error::{NodeError, NodeReason};
use crate::relay::CtrlSignal;
use crate::WorkerShutdownPriority;
use core::default::Default;
use core::fmt::Debug;
use core::sync::atomic::{AtomicUsize, Ordering};
use ockam_core::compat::collections::hash_map::Entry;
use ockam_core::compat::sync::Mutex as SyncMutex;
use ockam_core::compat::sync::RwLock as SyncRwLock;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{
    compat::{
        collections::{HashMap, HashSet},
        sync::Arc,
        vec::Vec,
    },
    flow_control::FlowControls,
    Address, AddressMetadata, Error, Mailbox, Mailboxes, RelayMessage, Result,
};

#[derive(Default)]
struct AddressMaps {
    // NOTE: It's crucial that if more that one of these structures is needed to perform an
    // operation, we should always acquire locks in the order they're declared here. Otherwise, it
    // can cause a deadlock.
    /// Registry of primary address to worker address record state
    records: SyncRwLock<HashMap<Address, AddressRecord>>,
    /// Alias-registry to map arbitrary address to primary addresses
    aliases: SyncRwLock<HashMap<Address, Address>>,
    /// Registry of arbitrary metadata for each address, lazily populated
    metadata: SyncRwLock<HashMap<Address, AddressMetadata>>,
}

/// Address states and associated logic
pub struct InternalMap {
    // NOTE: It's crucial that if more that one of these structures is needed to perform an
    // operation, we should always acquire locks in the order they're declared here. Otherwise, it
    // can cause a deadlock.
    address_maps: AddressMaps,
    /// Track non-detached addresses that are being stopped (except those that are stopped due to node shutdown)
    stopping: SyncMutex<HashSet<Address>>,
    /// Track non-detached addresses that are being stopped due to node shutdown
    stopping_shutdown: SyncMutex<HashSet<Address>>,
    /// Channel to notify when stopping_shutdown map gets empty
    shutdown_yield_sender: SyncMutex<Option<OneshotSender<()>>>,
    /// Access to [`FlowControls`] to clean resources
    flow_controls: FlowControls,
    /// Metrics collection and sharing
    #[cfg(feature = "metrics")]
    metrics: (Arc<AtomicUsize>, Arc<AtomicUsize>),
}

impl InternalMap {
    pub(crate) fn resolve(&self, addr: &Address) -> Result<MessageSender<RelayMessage>> {
        let records = self.address_maps.records.read().unwrap();
        let aliases = self.address_maps.aliases.read().unwrap();

        let address_record = if let Some(primary_address) = aliases.get(addr) {
            records.get(primary_address)
        } else {
            trace!("Resolving worker address '{addr}'... FAILED; no such alias");
            return Err(Error::new(
                Origin::Node,
                Kind::NotFound,
                format!("No such alias: {}", addr),
            ));
        };

        match address_record {
            Some(address_record) => {
                trace!("Resolving worker address '{addr}'... OK");
                address_record.increment_msg_count();
                Ok(address_record.sender.clone())
            }
            None => {
                trace!("Resolving worker address '{addr}'... FAILED; no such worker");
                Err(Error::new(
                    Origin::Node,
                    Kind::NotFound,
                    format!("No such address: {}", addr),
                ))
            }
        }
    }
}

impl InternalMap {
    pub(super) fn new(flow_controls: &FlowControls) -> Self {
        Self {
            address_maps: Default::default(),
            stopping: Default::default(),
            stopping_shutdown: Default::default(),
            shutdown_yield_sender: Default::default(),
            flow_controls: flow_controls.clone(),
            #[cfg(feature = "metrics")]
            metrics: Default::default(),
        }
    }
}

impl InternalMap {
    pub(super) fn stop(&self, address: &Address, skip_sending_stop_signal: bool) -> Result<()> {
        // To guarantee consistency we'll first acquire lock on all the maps we need to touch
        // and only then start modifications
        let mut records = self.address_maps.records.write().unwrap();
        let mut aliases = self.address_maps.aliases.write().unwrap();
        let mut metadata = self.address_maps.metadata.write().unwrap();
        let mut stopping = self.stopping.lock().unwrap();

        let primary_address = aliases
            .get(address)
            .ok_or_else(|| {
                Error::new(
                    Origin::Node,
                    Kind::NotFound,
                    format!("No such alias: {}", address),
                )
            })?
            .clone();

        self.flow_controls.cleanup_address(&primary_address);

        let record = if let Some(record) = records.remove(&primary_address) {
            record
        } else {
            return Err(Error::new(
                Origin::Node,
                Kind::NotFound,
                format!("No such address: {}", primary_address),
            ));
        };

        for address in &record.additional_addresses {
            metadata.remove(address);
            aliases.remove(address);
        }

        metadata.remove(&primary_address);
        aliases.remove(&primary_address);

        // Detached doesn't need any stop confirmation, since they don't have a Relay = don't have
        // an async task running in a background that should be stopped.
        if !record.meta.detached {
            let res = stopping.insert(primary_address);
            debug!(
                "Inserted {} into stopping. Inserted = {}",
                record.primary_address, res
            );
        }

        record.stop(skip_sending_stop_signal)?;

        Ok(())
    }

    pub(super) fn stop_ack(&self, primary_address: &Address) {
        {
            let mut stopping = self.stopping.lock().unwrap();
            let res = stopping.remove(primary_address);

            debug!(
                "Removing {} from stopping. Removed = {}",
                primary_address, res
            );
        }

        let mut stopping_shutdown = self.stopping_shutdown.lock().unwrap();

        let res = stopping_shutdown.remove(primary_address);
        debug!(
            "Removing {} from stopping_shutdown. Removed = {}",
            primary_address, res
        );

        if stopping_shutdown.is_empty() {
            if let Some(shutdown_yield_sender) = self.shutdown_yield_sender.lock().unwrap().take() {
                debug!("Sending stop_ack signal");
                if shutdown_yield_sender.send(()).is_err() {
                    warn!("shutdown_yield send errored");
                }
            }
        }
    }

    pub(super) fn is_worker_registered_at(&self, primary_address: &Address) -> bool {
        self.address_maps
            .records
            .read()
            .unwrap()
            .contains_key(primary_address)
        // TODO: we should also check aliases
    }

    pub(super) fn list_workers(&self) -> Vec<Address> {
        self.address_maps
            .records
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    pub(super) fn insert_address_record(
        &self,
        record: AddressRecord,
        mailboxes: &Mailboxes,
    ) -> Result<()> {
        let mut records = self.address_maps.records.write().unwrap();

        let entry = records.entry(record.primary_address.clone());

        let entry = match entry {
            Entry::Occupied(_) => {
                let node = NodeError::Address(record.primary_address);
                return Err(node.already_exists());
            }
            Entry::Vacant(entry) => entry,
        };

        // It may fail, so we don't insert record before that
        Self::insert_aliases(&mut self.address_maps.aliases.write().unwrap(), &record)?;
        Self::insert_all_metadata(&mut self.address_maps.metadata.write().unwrap(), mailboxes);

        entry.insert(record);

        Ok(())
    }

    fn insert_aliases(
        aliases: &mut HashMap<Address, Address>,
        record: &AddressRecord,
    ) -> Result<()> {
        Self::insert_alias(aliases, &record.primary_address, &record.primary_address)?;

        for i in 0..record.additional_addresses.len() {
            match Self::insert_alias(
                aliases,
                &record.primary_address,
                &record.additional_addresses[i],
            ) {
                Ok(_) => {}
                Err(err) => {
                    // Rollback
                    for j in 0..i {
                        aliases.remove(&record.additional_addresses[j]);
                    }

                    return Err(err);
                }
            }
        }

        Ok(())
    }

    fn insert_alias(
        aliases: &mut HashMap<Address, Address>,
        primary_address: &Address,
        alias: &Address,
    ) -> Result<()> {
        match aliases.insert(alias.clone(), primary_address.clone()) {
            None => Ok(()),
            Some(old_value) => {
                // Rollback
                aliases.insert(alias.clone(), old_value);

                let node = NodeError::Address(primary_address.clone());
                Err(node.already_exists())
            }
        }
    }

    fn insert_all_metadata(
        metadata: &mut HashMap<Address, AddressMetadata>,
        mailboxes: &Mailboxes,
    ) {
        Self::insert_mailbox_metadata(metadata, mailboxes.primary_mailbox());

        for mailbox in mailboxes.additional_mailboxes() {
            Self::insert_mailbox_metadata(metadata, mailbox);
        }
    }

    fn insert_mailbox_metadata(
        metadata: &mut HashMap<Address, AddressMetadata>,
        mailbox: &Mailbox,
    ) {
        if let Some(meta) = mailbox.metadata().clone() {
            metadata.insert(mailbox.address().clone(), meta.clone());
        }
    }

    pub(super) fn find_terminal_address<'a>(
        &self,
        addresses: impl Iterator<Item = &'a Address>,
    ) -> Option<(&'a Address, AddressMetadata)> {
        let metadata = self.address_maps.metadata.read().unwrap();
        for address in addresses {
            if let Some(metadata) = metadata.get(address) {
                if metadata.is_terminal {
                    return Some((address, metadata.clone()));
                }
            }
        }

        None
    }

    pub(super) fn get_address_metadata(&self, address: &Address) -> Option<AddressMetadata> {
        self.address_maps
            .metadata
            .read()
            .unwrap()
            .get(address)
            .cloned()
    }
}

impl InternalMap {
    #[cfg(feature = "metrics")]
    pub(super) fn update_metrics(&self) {
        self.metrics.0.store(
            self.address_maps.records.read().unwrap().len(),
            Ordering::Release,
        );
    }

    #[cfg(feature = "metrics")]
    pub(super) fn get_metrics(&self) -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
        (Arc::clone(&self.metrics.0), Arc::clone(&self.metrics.1))
    }

    #[cfg(feature = "metrics")]
    pub(super) fn get_addr_count(&self) -> usize {
        self.metrics.0.load(Ordering::Acquire)
    }

    /// Stop all workers with given priority
    pub(super) fn stop_workers(
        &self,
        shutdown_priority: WorkerShutdownPriority,
    ) -> Option<OneshotReceiver<()>> {
        let records_to_stop: Vec<AddressRecord> = {
            let mut records = self.address_maps.records.write().unwrap();

            // we remove address records, so workers to be stopped can no longer be found, therefore
            // can't be used to send messages
            records
                .extract_if(|_addr, record| record.shutdown_order == shutdown_priority)
                .map(|(_addr, record)| record)
                .collect()
        };

        let mut stopping_shutdown = self.stopping_shutdown.lock().unwrap();

        if !stopping_shutdown.is_empty() {
            warn!(
                "stopping_shutdown map is not empty, while next priority is about to be stopped. Clearing. Current priority: {:?}", shutdown_priority
            );
            stopping_shutdown.clear();
        }

        for record in records_to_stop {
            // Detached doesn't need any stop confirmation, since they don't have a Relay => they
            // don't have an async task running in a background that should be stopped.
            let primary_address = record.primary_address.clone();
            if !record.meta.detached {
                debug!("Inserted {} into stopping_shutdown", record.primary_address);
                stopping_shutdown.insert(primary_address.clone());
            }

            if let Err(err) = record.stop(false) {
                error!("Error stopping address. Err={}", err);
                // Let's not expect stop_ack from that worker in this case
                stopping_shutdown.remove(&primary_address);
            }
        }

        if !stopping_shutdown.is_empty() {
            // If we just stopped some non-detached workers, let's wait for stop_ack form all of them
            let (shutdown_yield_sender, shutdown_yield_receiver) = oneshot_channel();

            *self.shutdown_yield_sender.lock().unwrap() = Some(shutdown_yield_sender);

            Some(shutdown_yield_receiver)
        } else {
            None
        }
    }

    pub(super) fn force_clear_records(&self) -> Vec<Address> {
        let mut records = self.address_maps.records.write().unwrap();

        records.drain().map(|(address, _record)| address).collect()
    }
}

/// Additional metadata for worker records
#[derive(Debug)]
pub struct WorkerMeta {
    #[allow(dead_code)]
    pub processor: bool,
    pub detached: bool,
}

pub struct AddressRecord {
    primary_address: Address,
    additional_addresses: Vec<Address>,
    sender: MessageSender<RelayMessage>,
    ctrl_tx: OneshotSender<CtrlSignal>,
    meta: WorkerMeta,
    shutdown_order: WorkerShutdownPriority,
    msg_count: Arc<AtomicUsize>,
}

impl Debug for AddressRecord {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AddressRecord")
            .field("primary_address", &self.primary_address)
            .field("additional_addresses", &self.additional_addresses)
            .field("sender", &self.sender)
            .field("ctrl_tx", &self.ctrl_tx)
            .field("meta", &self.meta)
            .field("msg_count", &self.msg_count)
            .finish()
    }
}

impl AddressRecord {
    pub fn new(
        primary_address: Address,
        additional_addresses: Vec<Address>,
        sender: MessageSender<RelayMessage>,
        ctrl_tx: OneshotSender<CtrlSignal>,
        meta: WorkerMeta,
        shutdown_order: WorkerShutdownPriority,
        msg_count: Arc<AtomicUsize>,
    ) -> Self {
        AddressRecord {
            primary_address,
            additional_addresses,
            sender,
            ctrl_tx,
            meta,
            shutdown_order,
            msg_count,
        }
    }

    #[inline]
    pub fn increment_msg_count(&self) {
        self.msg_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Signal this worker to stop -- it will no longer be able to receive messages
    pub fn stop(self, skip_sending_stop_signal: bool) -> Result<()> {
        trace!("AddressRecord::stop called for {:?}", self.primary_address);

        if !self.meta.detached && !skip_sending_stop_signal {
            self.ctrl_tx
                .send(CtrlSignal::InterruptStop)
                .map_err(|_| NodeError::NodeState(NodeReason::Unknown).internal())?;
        }

        Ok(())
    }
}
