use crate::nodes::service::Alias;
use ockam::identity::Identifier;
use ockam::identity::{SecureChannel, SecureChannelListener};
use ockam::remote::RemoteRelayInfo;
use ockam_core::compat::collections::BTreeMap;
use ockam_core::{Address, Route};
use ockam_node::compat::asynchronous::RwLock;
use std::borrow::Borrow;
use std::fmt::Display;
use std::net::SocketAddr;

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

#[derive(Clone)]
pub struct SecureChannelListenerInfo {
    listener: SecureChannelListener,
}

impl SecureChannelListenerInfo {
    pub fn new(listener: SecureChannelListener) -> Self {
        Self { listener }
    }

    pub fn listener(&self) -> &SecureChannelListener {
        &self.listener
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
    Consumer,
    Producer,
    Outlet,
    Direct,
}

impl Display for KafkaServiceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KafkaServiceKind::Consumer => write!(f, "consumer"),
            KafkaServiceKind::Producer => write!(f, "producer"),
            KafkaServiceKind::Outlet => write!(f, "outlet"),
            KafkaServiceKind::Direct => write!(f, "direct"),
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
    pub(crate) worker_addr: Address,
    pub(crate) outlet_route: Route,
}

impl InletInfo {
    pub(crate) fn new(
        bind_addr: &str,
        worker_addr: Option<&Address>,
        outlet_route: &Route,
    ) -> Self {
        let worker_addr = match worker_addr {
            Some(addr) => addr.clone(),
            None => Address::from_string(""),
        };
        Self {
            bind_addr: bind_addr.to_owned(),
            worker_addr,
            outlet_route: outlet_route.to_owned(),
        }
    }
}

#[derive(Clone)]
pub struct OutletInfo {
    pub(crate) socket_addr: SocketAddr,
    pub(crate) worker_addr: Address,
}

impl OutletInfo {
    pub(crate) fn new(socket_addr: &SocketAddr, worker_addr: Option<&Address>) -> Self {
        let worker_addr = match worker_addr {
            Some(addr) => addr.clone(),
            None => Address::from_string(""),
        };
        Self {
            socket_addr: *socket_addr,
            worker_addr,
        }
    }
}

#[derive(Default)]
pub(crate) struct Registry {
    pub(crate) secure_channels: SecureChannelRegistry,
    pub(crate) secure_channel_listeners: RegistryOf<Address, SecureChannelListenerInfo>,
    pub(crate) uppercase_services: RegistryOf<Address, UppercaseServiceInfo>,
    pub(crate) echoer_services: RegistryOf<Address, EchoerServiceInfo>,
    pub(crate) kafka_services: RegistryOf<Address, KafkaServiceInfo>,
    pub(crate) hop_services: RegistryOf<Address, HopServiceInfo>,
    pub(crate) relays: RegistryOf<String, RemoteRelayInfo>,
    pub(crate) inlets: RegistryOf<Alias, InletInfo>,
    pub(crate) outlets: RegistryOf<Alias, OutletInfo>,
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
