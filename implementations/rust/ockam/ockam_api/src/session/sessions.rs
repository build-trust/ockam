use core::fmt;
use core::future::Future;
use core::pin::Pin;
use std::time::Duration;

use minicbor::bytes::ByteArray;
use minicbor::{Decode, Encode};
use tracing as log;

use ockam_core::compat::collections::HashMap;
use ockam_core::compat::rand;
use ockam_core::{Error, Route};

//most sessions replacer are dependent on the node manager, if many session
//fails concurrently, which is the common scenario we need extra time
//to account for the lock contention
pub const MAX_RECOVERY_TIME: Duration = Duration::from_secs(30);
pub const MAX_CONNECT_TIME: Duration = Duration::from_secs(15);

pub type Replacement = Pin<Box<dyn Future<Output = Result<Route, Error>> + Send>>;
pub type Replacer = Box<dyn FnMut(Route) -> Replacement + Send>;

#[derive(Debug)]
pub struct Sessions {
    map: HashMap<Key, Session>,
}

pub struct Session {
    key: Key,
    ping_route: Route,
    status: Status,
    replace: Replacer,
    pings: Vec<Ping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Down,
    Degraded,
    Up,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("key", &self.key)
            .field("ping_route", &self.ping_route)
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
            addr = %s.ping_route(),
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
    pub fn new(ping_route: Route) -> Self {
        Self {
            key: Key::new(),
            ping_route,
            status: Status::Up,
            replace: Box::new(move |r| Box::pin(async move { Ok(r) })),
            pings: Vec::new(),
        }
    }

    pub fn key(&self) -> Key {
        self.key
    }

    pub fn ping_route(&self) -> &Route {
        &self.ping_route
    }

    pub fn set_ping_address(&mut self, ping_route: Route) {
        self.ping_route = ping_route;
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn set_status(&mut self, s: Status) {
        self.status = s
    }

    pub fn replacement(&mut self, ping_route: Route) -> Replacement {
        (self.replace)(ping_route)
    }

    pub fn set_replacer(&mut self, f: Replacer) {
        self.replace = f
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
        write!(f, "{}", hex::encode(*self.0))
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
