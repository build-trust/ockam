use core::fmt;
use core::future::Future;
use core::pin::Pin;
use std::fmt::Formatter;
use std::time::Duration;

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use ockam_core::compat::rand;
use ockam_core::{Error, Route};

//most sessions replacer are dependent on the node manager, if many session
//fails concurrently, which is the common scenario we need extra time
//to account for the lock contention
pub const MAX_RECOVERY_TIME: Duration = Duration::from_secs(30);
pub const MAX_CONNECT_TIME: Duration = Duration::from_secs(15);

pub type Replacement = Pin<Box<dyn Future<Output = Result<Route, Error>> + Send>>;
pub type Replacer = Box<dyn FnMut(Route) -> Replacement + Send>;

pub struct Session {
    key: String,
    ping_route: Route,
    status: ConnectionStatus,
    replace: Replacer,
    pings: Vec<Ping>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
pub enum ConnectionStatus {
    #[n(0)]
    Down,
    #[n(1)]
    Degraded,
    #[n(2)]
    Up,
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Down => write!(f, "down"),
            ConnectionStatus::Degraded => write!(f, "degraded"),
            ConnectionStatus::Up => write!(f, "up"),
        }
    }
}

impl TryFrom<String> for ConnectionStatus {
    type Error = ApiError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "down" => Ok(ConnectionStatus::Down),
            "degraded" => Ok(ConnectionStatus::Degraded),
            "up" => Ok(ConnectionStatus::Up),
            _ => Err(ApiError::message(format!(
                "Invalid connection status: {value}"
            ))),
        }
    }
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

impl Session {
    /// Create a new session
    ///
    /// # Arguments
    ///
    /// * `ping_route` - The route to send pings to
    /// * `key` - The key to identify the session, usually adding the kind of the session
    ///           with the key used within the service registry is the way to gos
    pub fn new(ping_route: Route, key: String) -> Self {
        Self {
            key,
            ping_route,
            status: ConnectionStatus::Up,
            replace: Box::new(move |r| Box::pin(async move { Ok(r) })),
            pings: Vec::new(),
        }
    }

    pub fn key(&self) -> &str {
        self.key.as_str()
    }

    pub fn ping_route(&self) -> &Route {
        &self.ping_route
    }

    pub fn set_ping_address(&mut self, ping_route: Route) {
        self.ping_route = ping_route;
    }

    pub fn status(&self) -> ConnectionStatus {
        self.status
    }

    pub fn set_status(&mut self, s: ConnectionStatus) {
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
