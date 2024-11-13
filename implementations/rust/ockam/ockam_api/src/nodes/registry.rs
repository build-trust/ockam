use crate::cli_state::random_name;
use crate::DefaultAddress;

use ockam::identity::Identifier;
use ockam::identity::{SecureChannel, SecureChannelListener};
use ockam_core::compat::collections::hash_map::Equivalent;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::RwLock as SyncRwLock;
use ockam_core::{Address, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::asynchronous::Mutex as AsyncMutex;
use ockam_transport_core::HostnamePort;

use crate::session::session::Session;
use std::fmt::Display;
use std::hash::Hash;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct SecureChannelRegistry {
    channels: SyncRwLock<Vec<SecureChannelInfo>>,
}

impl SecureChannelRegistry {
    pub fn get_by_addr(&self, addr: &Address) -> Option<SecureChannelInfo> {
        self.channels
            .read()
            .unwrap()
            .iter()
            .find(|&x| x.sc.encryptor_address() == addr)
            .cloned()
    }

    pub fn insert(
        &self,
        route: Route,
        sc: SecureChannel,
        authorized_identifiers: Option<Vec<Identifier>>,
    ) {
        self.channels.write().unwrap().push(SecureChannelInfo::new(
            route,
            sc,
            authorized_identifiers,
        ))
    }

    pub fn remove_by_addr(&self, addr: &Address) {
        self.channels
            .write()
            .unwrap()
            .retain(|x| x.sc().encryptor_address() != addr)
    }

    pub fn list(&self) -> Vec<SecureChannelInfo> {
        self.channels.read().unwrap().clone()
    }
}

#[derive(Clone)]
pub struct SecureChannelInfo {
    // Target route of the channel
    route: Route,
    sc: SecureChannel,
    authorized_identifiers: Option<Vec<Identifier>>,
}

impl SecureChannelInfo {
    pub fn new(
        route: Route,
        sc: SecureChannel,
        authorized_identifiers: Option<Vec<Identifier>>,
    ) -> Self {
        Self {
            route,
            sc,
            authorized_identifiers,
        }
    }

    pub fn route(&self) -> &Route {
        &self.route
    }

    pub fn sc(&self) -> &SecureChannel {
        &self.sc
    }

    pub fn authorized_identifiers(&self) -> Option<&Vec<Identifier>> {
        self.authorized_identifiers.as_ref()
    }
}

#[derive(Default, Clone)]
pub(crate) struct UppercaseServiceInfo {}

#[derive(Default, Clone)]
pub(crate) struct EchoerServiceInfo {}

#[derive(Default, Clone)]
pub(crate) struct HopServiceInfo {}

#[derive(Eq, PartialEq, Clone)]
pub enum KafkaServiceKind {
    Inlet,
    Outlet,
}

impl Display for KafkaServiceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KafkaServiceKind::Inlet => write!(f, "inlet"),
            KafkaServiceKind::Outlet => write!(f, "outlet"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct KafkaServiceInfo {
    kind: KafkaServiceKind,
}

impl KafkaServiceInfo {
    pub fn new(kind: KafkaServiceKind) -> Self {
        Self { kind }
    }

    pub fn kind(&self) -> &KafkaServiceKind {
        &self.kind
    }
}

#[derive(Clone)]
pub(crate) struct InletInfo {
    pub(crate) bind_addr: String,
    pub(crate) outlet_addr: MultiAddr,
    pub(crate) session: Arc<AsyncMutex<Session>>,
    pub(crate) privileged: bool,
}

impl InletInfo {
    pub(crate) fn new(
        bind_addr: &str,
        outlet_addr: MultiAddr,
        session: Session,
        privileged: bool,
    ) -> Self {
        Self {
            bind_addr: bind_addr.to_owned(),
            outlet_addr,
            session: Arc::new(AsyncMutex::new(session)),
            privileged,
        }
    }
}

#[derive(Clone)]
pub struct OutletInfo {
    pub(crate) to: HostnamePort,
    pub(crate) worker_addr: Address,
    pub(crate) privileged: bool,
}

impl OutletInfo {
    pub(crate) fn new(to: HostnamePort, worker_addr: Option<&Address>, privileged: bool) -> Self {
        let worker_addr = match worker_addr {
            Some(addr) => addr.clone(),
            None => Address::from_string(""),
        };
        Self {
            to,
            worker_addr,
            privileged,
        }
    }
}

#[derive(Clone)]
pub struct RegistryRelayInfo {
    pub(crate) destination_address: MultiAddr,
    pub(crate) alias: String,
    pub(crate) session: Arc<AsyncMutex<Session>>,
}

#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) secure_channels: SecureChannelRegistry,
    pub(crate) secure_channel_listeners: RegistryOf<Address, SecureChannelListener>,
    pub(crate) uppercase_services: RegistryOf<Address, UppercaseServiceInfo>,
    pub(crate) echoer_services: RegistryOf<Address, EchoerServiceInfo>,
    pub(crate) kafka_services: RegistryOf<Address, KafkaServiceInfo>,
    pub(crate) hop_services: RegistryOf<Address, HopServiceInfo>,
    pub(crate) relays: RegistryOf<String, RegistryRelayInfo>,
    pub(crate) inlets: RegistryOf<String, InletInfo>,
    pub(crate) outlets: RegistryOf<Address, OutletInfo>,
    pub(crate) influxdb_services: RegistryOf<Address, ()>, // TODO: what should we persist here?
}

pub(crate) struct RegistryOf<K, V> {
    map: SyncRwLock<HashMap<K, V>>,
}

impl<K, V> Default for RegistryOf<K, V> {
    fn default() -> Self {
        RegistryOf {
            map: Default::default(),
        }
    }
}

impl<K: Hash + Eq + Clone, V: Clone> RegistryOf<K, V> {
    pub fn insert(&self, k: K, v: V) -> Option<V> {
        self.map.write().unwrap().insert(k, v)
    }

    pub fn get<Q>(&self, key: &Q) -> Option<V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.map.read().unwrap().get(key).cloned()
    }

    pub fn keys(&self) -> Vec<K> {
        self.map.read().unwrap().clone().keys().cloned().collect()
    }

    pub fn values(&self) -> Vec<V> {
        self.map.read().unwrap().values().cloned().collect()
    }

    pub fn entries(&self) -> Vec<(K, V)> {
        self.map
            .read()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn remove<Q>(&self, key: &Q) -> Option<V>
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.map.write().unwrap().remove(key)
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: Hash + Equivalent<K> + ?Sized,
    {
        self.map.read().unwrap().contains_key(key)
    }
}

impl RegistryOf<Address, OutletInfo> {
    pub fn generate_worker_addr(&self, worker_addr: Option<Address>) -> Address {
        match worker_addr {
            Some(addr) => addr,
            None => {
                // If no worker address is passed, return the default address if it's not in use
                let default: Address = DefaultAddress::OUTLET_SERVICE.into();
                if self.contains_key(&default) {
                    random_name().into()
                } else {
                    default
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outlet_registry_generate_worker_address_start_with_none() {
        let registry = Registry::default();

        // No worker address passed, should return the default address because it's not in use
        let worker_addr = registry.outlets.generate_worker_addr(None);
        assert_eq!(worker_addr, DefaultAddress::OUTLET_SERVICE.into());
        registry
            .outlets
            .insert(worker_addr.clone(), outlet_info(worker_addr));
        assert_eq!(registry.outlets.entries().len(), 1);

        // No worker address passed, should return a random address because the default it's in use
        let worker_addr = registry.outlets.generate_worker_addr(None);
        assert_ne!(worker_addr, DefaultAddress::OUTLET_SERVICE.into());
        registry
            .outlets
            .insert(worker_addr.clone(), outlet_info(worker_addr));
        assert_eq!(registry.outlets.entries().len(), 2);

        // Worker address passed, should return the same address
        let passed_addr = Address::from_string("my_outlet");
        let worker_addr = registry
            .outlets
            .generate_worker_addr(Some(passed_addr.clone()));
        assert_eq!(worker_addr, passed_addr.clone());
        registry
            .outlets
            .insert(worker_addr.clone(), outlet_info(worker_addr));
        assert_eq!(registry.outlets.entries().len(), 3);

        // Same worker address passed, should return the same address and not a random one
        let worker_addr = registry
            .outlets
            .generate_worker_addr(Some(passed_addr.clone()));
        assert_eq!(worker_addr, passed_addr.clone());
    }

    #[test]
    fn outlet_registry_generate_worker_address_start_with_some() {
        let registry = Registry::default();

        // Worker address passed, should return the same address
        let passed_addr = Address::from_string("my_outlet");
        let worker_addr = registry
            .outlets
            .generate_worker_addr(Some(passed_addr.clone()));
        assert_eq!(worker_addr, passed_addr);
        registry
            .outlets
            .insert(worker_addr.clone(), outlet_info(worker_addr));
        assert_eq!(registry.outlets.entries().len(), 1);

        // No worker address passed, should return the default address because it's not in use
        let worker_addr = registry.outlets.generate_worker_addr(None);
        assert_eq!(worker_addr, DefaultAddress::OUTLET_SERVICE.into());
        registry
            .outlets
            .insert(worker_addr.clone(), outlet_info(worker_addr));
        assert_eq!(registry.outlets.entries().len(), 2);

        // No worker address passed, should return a random address because the default it's in use
        let worker_addr = registry.outlets.generate_worker_addr(None);
        assert_ne!(worker_addr, DefaultAddress::OUTLET_SERVICE.into());
    }

    fn outlet_info(worker_addr: Address) -> OutletInfo {
        OutletInfo::new(
            HostnamePort::new("127.0.0.1", 0).unwrap(),
            Some(&worker_addr),
            true,
        )
    }
}
