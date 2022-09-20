use core::fmt;
use core::future::Future;
use core::pin::Pin;
use minicbor::{Decode, Encode};
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::rand;
use ockam_core::{Address, Error};
use tracing as log;

pub type Replacement = Pin<Box<dyn Future<Output = Result<Address, Error>> + Send>>;

#[derive(Debug)]
pub struct Sessions {
    ctr: u64,
    map: HashMap<Key, Session>,
}

pub struct Session {
    key: Key,
    address: Address,
    status: Status,
    replace: Box<dyn Fn(Address) -> Replacement + Send>,
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
            .field("address", &self.address)
            .field("status", &self.status)
            .field("pings", &self.pings)
            .finish()
    }
}

impl Sessions {
    pub fn new() -> Self {
        Self {
            ctr: 0,
            map: HashMap::new(),
        }
    }

    pub fn add(&mut self, mut s: Session) -> Key {
        let k = Key::new(self.ctr);
        self.ctr += 1;
        s.key = k;
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
    pub fn new(addr: Address) -> Self {
        Self {
            key: Key::default(),
            address: addr,
            status: Status::Up,
            replace: Box::new(move |addr| Box::pin(async move { Ok(addr) })),
            pings: Vec::new(),
        }
    }

    pub fn key(&self) -> Key {
        self.key
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn set_address(&mut self, a: Address) {
        self.address = a
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn set_status(&mut self, s: Status) {
        self.status = s
    }

    pub fn replacement(&self, addr: Address) -> Replacement {
        (self.replace)(addr)
    }

    pub fn set_replacement<F>(&mut self, f: F)
    where
        F: Fn(Address) -> Replacement + Send + 'static,
    {
        self.replace = Box::new(f)
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

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Encode, Decode)]
#[rustfmt::skip]
pub struct Key {
    #[n(0)] ctr: u64,
    #[n(1)] rnd: u32,
}

impl Key {
    fn new(n: u64) -> Self {
        Self {
            ctr: n,
            rnd: rand::random(),
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:x},{:x})", self.ctr, self.rnd)
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
