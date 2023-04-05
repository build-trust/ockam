//! Sessions
//!
//! Allows limiting messaging between workers for security reasons (Message Flow Authorization).
//!
//! Sessions imply 3 roles:
//!
//! - Producers, that produce messages (that usually originate on other nodes), which are
//!     potentially malicious and should be limited in terms of which workers they can reach.
//!     Tcp Receiver is an examples of a Producer.
//! - Consumers, that are allowed to consume potentially malicious messages from Producers.
//!     Secure Channel Decryptor is an example of Consumer.
//! - Spawners, that spawn Consumers and/or Producers
//!     Tcp Listener is an example of Spawner that spawns Producers
//!     Secure Channel Listener is an example of Spawner that spawns both Producers and Consumers.
//!
//! Each Session is identified by a unique random [`SessionId`].
//! Producers, Consumers and Spawners are identified by their messaging [`Address`].
//! [`Sessions`] object is used to store all Sessions-related data, as well as setup interactions
//! between Producers, Consumers and Spawners.

use crate::compat::collections::BTreeMap;
use crate::compat::rand::random;
use crate::compat::sync::{Arc, RwLock};
use crate::compat::vec::Vec;
use crate::Address;

mod access_control;
mod policy;
mod session_id;

pub use access_control::*;
pub use policy::*;
pub use session_id::*;

// TODO: Consider integrating this into Routing for better UX + to allow removing
// entries from that storage
/// Storage for all Session-related data
#[derive(Clone, Debug, Default)]
pub struct Sessions {
    // All known consumers
    consumers: Arc<RwLock<BTreeMap<SessionId, ConsumersInfo>>>,
    // All known producers
    producers: Arc<RwLock<BTreeMap<Address, ProducerInfo>>>,
    // Allows to find producer by having its additional Address,
    // e.g. Decryptor by its Encryptor Address or TCP Receiver by its TCP Sender Address
    producers_additional_addresses: Arc<RwLock<BTreeMap<Address, Address>>>,
    // All known spawners
    spawners: Arc<RwLock<BTreeMap<Address, SessionId>>>,
}

impl Sessions {
    /// Generate a fresh random [`SessionId`]
    pub fn generate_session_id(&self) -> SessionId {
        random()
    }

    /// Mark that given [`Address`] is a Consumer for Producer or Spawner with the given [`SessionId`]
    pub fn add_consumer(&self, address: &Address, session_id: &SessionId, policy: SessionPolicy) {
        let mut consumers = self.consumers.write().unwrap();
        if !consumers.contains_key(session_id) {
            consumers.insert(session_id.clone(), Default::default());
        }

        let session_consumers = consumers.get_mut(session_id).unwrap();

        session_consumers.0.insert(address.clone(), policy);
    }

    /// Mark that given [`Address`] is a Producer for to the given [`SessionId`]
    /// Also, mark that this Producer was spawned by a Spawner
    /// with the given spawner [`SessionId`] (if that's the case).
    pub fn add_producer(
        &self,
        address: &Address,
        session_id: &SessionId,
        spawner_session_id: Option<&SessionId>,
        additional_addresses: Vec<Address>,
    ) {
        let mut producers = self.producers.write().unwrap();
        producers.insert(
            address.clone(),
            ProducerInfo {
                session_id: session_id.clone(),
                spawner_session_id: spawner_session_id.cloned(),
            },
        );
        drop(producers);

        let mut producers_additional_addresses =
            self.producers_additional_addresses.write().unwrap();
        producers_additional_addresses.insert(address.clone(), address.clone());
        for additional_address in additional_addresses {
            producers_additional_addresses.insert(additional_address, address.clone());
        }
    }

    /// Mark that given [`Address`] is a Spawner for to the given [`SessionId`]
    pub fn add_spawner(&self, address: &Address, session_id: &SessionId) {
        let mut spawners = self.spawners.write().unwrap();

        spawners.insert(address.clone(), session_id.clone());
    }

    /// Get known Consumers for the given [`SessionId`]
    pub fn get_consumers_info(&self, session_id: &SessionId) -> ConsumersInfo {
        let consumers = self.consumers.read().unwrap();
        consumers.get(session_id).cloned().unwrap_or_default()
    }

    /// Get [`SessionId`] for which given [`Address`] is a Spawner
    pub fn get_session_with_spawner(&self, address: &Address) -> Option<SessionId> {
        let spawners = self.spawners.read().unwrap();
        spawners.get(address).cloned()
    }

    /// Get [`SessionId`] for which given [`Address`] is a Producer
    pub fn get_session_with_producer(&self, address: &Address) -> Option<ProducerInfo> {
        let producers = self.producers.read().unwrap();
        producers.get(address).cloned()
    }

    /// Get [`SessionId`] for which given [`Address`] is a Producer or is an additional [`Address`]
    /// fot that Producer (e.g. Encryptor address for its Decryptor, or TCP Sender for its TCP Receiver)
    pub fn find_session_with_producer_address(&self, address: &Address) -> Option<ProducerInfo> {
        let producers_additional_addresses = self.producers_additional_addresses.read().unwrap();
        let producer_address = match producers_additional_addresses.get(address) {
            Some(address) => address.clone(),
            None => return None,
        };
        drop(producers_additional_addresses);
        let producers = self.producers.read().unwrap();
        producers.get(&producer_address).cloned()
    }

    /// Get all [`SessionId`]s for which given [`Address`] is a Consumer
    pub fn get_sessions_with_consumer(&self, address: &Address) -> Vec<SessionId> {
        let consumers = self.consumers.read().unwrap();
        consumers
            .iter()
            .filter(|&x| x.1 .0.contains_key(address))
            .map(|x| x.0.clone())
            .collect()
    }
}

/// Known Consumers for the given [`SessionId`]
#[derive(Default, Clone, Debug)]
pub struct ConsumersInfo(BTreeMap<Address, SessionPolicy>);

/// Producer information
#[derive(Clone, Debug)]
pub struct ProducerInfo {
    session_id: SessionId,
    spawner_session_id: Option<SessionId>,
}

impl ProducerInfo {
    /// [`SessionId`]
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    /// Spawner's [`SessionId`]
    pub fn spawner_session_id(&self) -> &Option<SessionId> {
        &self.spawner_session_id
    }
}
