use core::any::Any;
use core::fmt;
use core::future::Future;
use core::pin::Pin;
use minicbor::bytes::ByteArray;
use minicbor::{Decode, Encode};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::rand;
use ockam_core::Error;
use ockam_multiaddr::MultiAddr;
use tracing as log;

pub type Replacement = Pin<Box<dyn Future<Output = Result<MultiAddr, Error>> + Send>>;

#[derive(Debug)]
pub struct Sessions {
    map: HashMap<Key, Session>,
}

pub struct Session {
    key: Key,
    addr: MultiAddr,
    meta: HashMap<&'static str, Box<dyn Any + Send>>,
    status: Status,
    replace: Box<dyn Fn(MultiAddr) -> Replacement + Send>,
    pings: Vec<Ping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Down,
    Up,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("key", &self.key)
            .field("addr", &self.addr)
            .field("status", &self.status)
            .field("pings", &self.pings)
            .finish()
    }
}

impl Sessions {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn add(&mut self, s: Session) -> Key {
        let k = s.key();
        log::debug! {
            target: "ockam_api::session",
            key = %k,
            addr = %s.address(),
            "session added"
        }
        self.map.insert(k, s);
        k
    }

    #[allow(unused)]
    pub fn session(&self, k: &Key) -> Option<&Session> {
        self.map.get(k)
    }

    pub fn session_mut(&mut self, k: &Key) -> Option<&mut Session> {
        self.map.get_mut(k)
    }

    #[allow(unused)]
    pub fn iter(&self) -> impl Iterator<Item = (&Key, &Session)> + '_ {
        self.map.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Key, &mut Session)> + '_ {
        self.map.iter_mut()
    }
}

impl Session {
    pub fn new(addr: MultiAddr) -> Self {
        Self {
            key: Key::new(),
            addr,
            meta: HashMap::new(),
            status: Status::Up,
            replace: Box::new(move |r| Box::pin(async move { Ok(r) })),
            pings: Vec::new(),
        }
    }

    pub fn key(&self) -> Key {
        self.key
    }

    pub fn address(&self) -> &MultiAddr {
        &self.addr
    }

    pub fn set_address(&mut self, a: MultiAddr) {
        self.addr = a
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn set_status(&mut self, s: Status) {
        self.status = s
    }

    pub fn replacement(&self, a: MultiAddr) -> Replacement {
        (self.replace)(a)
    }

    pub fn set_replacement<F>(&mut self, f: F)
    where
        F: Fn(MultiAddr) -> Replacement + Send + 'static,
    {
        self.replace = Box::new(f)
    }

    pub fn put<T: Send + 'static>(&mut self, key: &'static str, data: T) {
        self.meta.insert(key, Box::new(data));
    }

    pub fn get<T: 'static>(&self, key: &str) -> Option<&T> {
        self.meta.get(key).and_then(|data| data.downcast_ref())
    }

    pub fn pings(&self) -> &[Ping] {
        &self.pings
    }

    pub fn add_ping(&mut self, p: Ping) {
        self.pings.push(p);
    }

    pub fn clear_pings(&mut self) {
        self.pings.clear()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Encode, Decode)]
#[rustfmt::skip]
pub struct Key(#[n(0)] ByteArray<24>);

impl Key {
    fn new() -> Self {
        Self(rand::random::<[u8; 24]>().into())
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&*self.0))
    }
}

#[derive(Debug, Default, Copy, Clone, Encode, Decode, PartialEq, Eq)]
#[cbor(transparent)]
pub struct Ping(#[n(0)] u64);

impl Ping {
    pub fn new() -> Self {
        Self(rand::random())
    }
}

impl fmt::Display for Ping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}
