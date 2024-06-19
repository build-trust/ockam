use crate::cli_state::random_name;
use crate::nodes::models::relay::RelayInfo;
use crate::session::sessions::{ReplacerOutputKind, Session};
use crate::DefaultAddress;
use ockam::identity::Identifier;
use ockam::identity::{SecureChannel, SecureChannelListener};
use ockam_core::compat::collections::BTreeMap;
use ockam_core::{Address, Route};
use ockam_multiaddr::MultiAddr;
use ockam_node::compat::asynchronous::RwLock;
use ockam_node::HostnamePort;
use std::borrow::Borrow;
use std::fmt::Display;

#[derive(Default)]
pub(crate) struct SecureChannelRegistry {
    channels: RwLock<Vec<SecureChannelInfo>>,
}

impl SecureChannelRegistry {
    pub async fn get_by_addr(&self, addr: &Address) -> Option<SecureChannelInfo> {
        let channels = self.channels.read().await;
        channels
            .iter()
            .find(|&x| x.sc.encryptor_address() == addr)
            .cloned()
    }

    pub async fn insert(
        &self,
        route: Route,
        sc: SecureChannel,
        authorized_identifiers: Option<Vec<Identifier>>,
    ) {
        let mut channels = self.channels.write().await;
        channels.push(SecureChannelInfo::new(route, sc, authorized_identifiers))
    }

    pub async fn remove_by_addr(&self, addr: &Address) {
        let mut channels = self.channels.write().await;
        channels.retain(|x| x.sc().encryptor_address() != addr)
    }

    pub async fn list(&self) -> Vec<SecureChannelInfo> {
        let channels = self.channels.read().await;
        channels.clone()
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
pub(crate) struct OktaIdentityProviderServiceInfo {}

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
    pub(crate) session: Session,
}

impl InletInfo {
    pub(crate) fn new(bind_addr: &str, outlet_addr: MultiAddr, session: Session) -> Self {
        Self {
            bind_addr: bind_addr.to_owned(),
            outlet_addr,
            session,
        }
    }
}

#[derive(Clone)]
pub struct OutletInfo {
    pub(crate) hostname_port: HostnamePort,
    pub(crate) worker_addr: Address,
}

impl OutletInfo {
    pub(crate) fn new(hostname_port: HostnamePort, worker_addr: Option<&Address>) -> Self {
        let worker_addr = match worker_addr {
            Some(addr) => addr.clone(),
            None => Address::from_string(""),
        };
        Self {
            hostname_port,
            worker_addr,
        }
    }
}

#[derive(Clone)]
pub struct RegistryRelayInfo {
    pub(crate) destination_address: MultiAddr,
    pub(crate) alias: String,
    pub(crate) session: Session,
}

impl From<RegistryRelayInfo> for RelayInfo {
    fn from(registry_relay_info: RegistryRelayInfo) -> Self {
        let relay_info = RelayInfo::new(
            registry_relay_info.destination_address.clone(),
            registry_relay_info.alias.clone(),
            registry_relay_info.session.connection_status(),
        );

        let current_relay_status =
            registry_relay_info
                .session
                .status()
                .map(|info| match info.kind {
                    ReplacerOutputKind::Inlet(_) => {
                        panic!("InletInfo should not be in the registry")
                    }
                    ReplacerOutputKind::Relay(info) => info,
                });

        if let Some(current_relay_status) = current_relay_status {
            relay_info.with(current_relay_status)
        } else {
            relay_info
        }
    }
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
}

pub(crate) struct RegistryOf<K, V> {
    map: RwLock<BTreeMap<K, V>>,
}

impl<K, V> Default for RegistryOf<K, V> {
    fn default() -> Self {
        RegistryOf {
            map: RwLock::new(BTreeMap::default()),
        }
    }
}

impl<K: Clone, V: Clone> RegistryOf<K, V> {
    pub async fn insert(&self, k: K, v: V) -> Option<V>
    where
        K: Ord,
    {
        let mut map = self.map.write().await;
        map.insert(k, v)
    }

    pub async fn get<Q: ?Sized>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        let map = self.map.read().await;
        map.get(key).cloned()
    }

    pub async fn keys(&self) -> Vec<K> {
        let map = self.map.read().await;
        map.clone().keys().cloned().collect()
    }

    pub async fn values(&self) -> Vec<V> {
        let map = self.map.read().await;
        map.clone().values().cloned().collect()
    }

    pub async fn entries(&self) -> Vec<(K, V)> {
        let map = self.map.read().await;
        map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    pub async fn remove<Q: ?Sized>(&self, key: &Q) -> Option<V>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        let mut map = self.map.write().await;
        map.remove(key)
    }

    pub async fn contains_key<Q: ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        let map = self.map.read().await;
        map.contains_key(key)
    }
}

impl RegistryOf<Address, OutletInfo> {
    pub async fn generate_worker_addr(&self, worker_addr: Option<Address>) -> Address {
        match worker_addr {
            Some(addr) => addr,
            None => {
                // If no worker address is passed, return the default address if it's not in use
                let default: Address = DefaultAddress::OUTLET_SERVICE.into();
                if self.contains_key(&default).await {
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
    use ockam_node::HostnamePort;

    #[tokio::test]
    async fn outlet_registry_generate_worker_address_start_with_none() {
        let registry = Registry::default();

        // No worker address passed, should return the default address because it's not in use
        let worker_addr = registry.outlets.generate_worker_addr(None).await;
        assert_eq!(worker_addr, DefaultAddress::OUTLET_SERVICE.into());
        registry
            .outlets
            .insert(worker_addr.clone(), outlet_info(worker_addr))
            .await;
        assert_eq!(registry.outlets.entries().await.len(), 1);

        // No worker address passed, should return a random address because the default it's in use
        let worker_addr = registry.outlets.generate_worker_addr(None).await;
        assert_ne!(worker_addr, DefaultAddress::OUTLET_SERVICE.into());
        registry
            .outlets
            .insert(worker_addr.clone(), outlet_info(worker_addr))
            .await;
        assert_eq!(registry.outlets.entries().await.len(), 2);

        // Worker address passed, should return the same address
        let passed_addr = Address::from_string("my_outlet");
        let worker_addr = registry
            .outlets
            .generate_worker_addr(Some(passed_addr.clone()))
            .await;
        assert_eq!(worker_addr, passed_addr.clone());
        registry
            .outlets
            .insert(worker_addr.clone(), outlet_info(worker_addr))
            .await;
        assert_eq!(registry.outlets.entries().await.len(), 3);

        // Same worker address passed, should return the same address and not a random one
        let worker_addr = registry
            .outlets
            .generate_worker_addr(Some(passed_addr.clone()))
            .await;
        assert_eq!(worker_addr, passed_addr.clone());
    }

    #[tokio::test]
    async fn outlet_registry_generate_worker_address_start_with_some() {
        let registry = Registry::default();

        // Worker address passed, should return the same address
        let passed_addr = Address::from_string("my_outlet");
        let worker_addr = registry
            .outlets
            .generate_worker_addr(Some(passed_addr.clone()))
            .await;
        assert_eq!(worker_addr, passed_addr);
        registry
            .outlets
            .insert(worker_addr.clone(), outlet_info(worker_addr))
            .await;
        assert_eq!(registry.outlets.entries().await.len(), 1);

        // No worker address passed, should return the default address because it's not in use
        let worker_addr = registry.outlets.generate_worker_addr(None).await;
        assert_eq!(worker_addr, DefaultAddress::OUTLET_SERVICE.into());
        registry
            .outlets
            .insert(worker_addr.clone(), outlet_info(worker_addr))
            .await;
        assert_eq!(registry.outlets.entries().await.len(), 2);

        // No worker address passed, should return a random address because the default it's in use
        let worker_addr = registry.outlets.generate_worker_addr(None).await;
        assert_ne!(worker_addr, DefaultAddress::OUTLET_SERVICE.into());
    }

    fn outlet_info(worker_addr: Address) -> OutletInfo {
        OutletInfo::new(HostnamePort::new("127.0.0.1", 0), Some(&worker_addr))
    }
}
