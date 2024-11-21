use super::{Code, Codec, Protocol};
use crate::codec::StdCodec;
use crate::proto::{DnsAddr, Node, Project, Secure, Service, Space, Tcp, Udp, Worker};
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use core::fmt;

#[derive(Clone)]
pub struct Registry {
    inner: Arc<RegistryImpl>,
}

struct RegistryImpl {
    bytes: BTreeMap<Code, Arc<dyn Codec>>,
    strings: BTreeMap<&'static str, Arc<dyn Codec>>,
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Registry")
    }
}

impl Default for Registry {
    fn default() -> Self {
        let std_codec = Arc::new(StdCodec);
        let mut r = RegistryBuilder::new();
        r.register(Worker::CODE, Worker::PREFIX, std_codec.clone());
        r.register(Tcp::CODE, Tcp::PREFIX, std_codec.clone());
        r.register(Udp::CODE, Udp::PREFIX, std_codec.clone());
        r.register(DnsAddr::CODE, DnsAddr::PREFIX, std_codec.clone());
        #[allow(clippy::redundant_clone)]
        r.register(Service::CODE, Service::PREFIX, std_codec.clone());
        #[allow(clippy::redundant_clone)]
        r.register(Node::CODE, Node::PREFIX, std_codec.clone());
        #[allow(clippy::redundant_clone)]
        r.register(Project::CODE, Project::PREFIX, std_codec.clone());
        #[allow(clippy::redundant_clone)]
        r.register(Space::CODE, Space::PREFIX, std_codec.clone());
        #[allow(clippy::redundant_clone)]
        r.register(Secure::CODE, Secure::PREFIX, std_codec.clone());
        #[cfg(feature = "std")]
        r.register(
            crate::proto::Ip4::CODE,
            crate::proto::Ip4::PREFIX,
            std_codec.clone(),
        )
        .register(
            crate::proto::Ip6::CODE,
            crate::proto::Ip6::PREFIX,
            std_codec,
        );
        r.finish()
    }
}

impl Registry {
    pub fn get_by_code(&self, code: Code) -> Option<Arc<dyn Codec>> {
        self.inner.bytes.get(&code).cloned()
    }

    pub fn get_by_prefix(&self, prefix: &str) -> Option<Arc<dyn Codec>> {
        self.inner.strings.get(prefix).cloned()
    }

    pub fn codes(&self) -> impl Iterator<Item = Code> + '_ {
        self.inner.bytes.keys().copied()
    }

    pub fn prefixes(&self) -> impl Iterator<Item = &str> + '_ {
        self.inner.strings.keys().copied()
    }
}

pub struct RegistryBuilder(RegistryImpl);

impl Default for RegistryBuilder {
    fn default() -> Self {
        RegistryBuilder::new()
    }
}

impl RegistryBuilder {
    pub fn new() -> Self {
        RegistryBuilder(RegistryImpl {
            bytes: BTreeMap::new(),
            strings: BTreeMap::new(),
        })
    }

    pub fn has_code(&self, c: Code) -> bool {
        self.0.bytes.contains_key(&c)
    }

    pub fn has_prefix(&self, prefix: &str) -> bool {
        self.0.strings.contains_key(prefix)
    }

    pub fn register<T>(&mut self, code: Code, prefix: &'static str, codec: Arc<T>) -> &mut Self
    where
        T: Codec + 'static,
    {
        self.0.bytes.insert(code, codec.clone());
        self.0.strings.insert(prefix, codec);
        self
    }

    pub fn finish(self) -> Registry {
        Registry {
            inner: Arc::new(self.0),
        }
    }
}
