use core::fmt;
use std::fmt::Formatter;
use std::sync::Arc;
use std::time::Duration;

use minicbor::{CborLen, Decode, Encode};
use ockam::remote::RemoteRelayInfo;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::colors::{color_error, color_ok, color_warn};
use crate::error::ApiError;
use ockam_core::compat::rand;
use ockam_core::{async_trait, Address, Error, Route};
use rand::random;

//most sessions replacer are dependent on the node manager, if many session
//fails concurrently, which is the common scenario we need extra time
//to account for the lock contention
pub const MAX_RECOVERY_TIME: Duration = Duration::from_secs(30);
pub const MAX_CONNECT_TIME: Duration = Duration::from_secs(15);

#[async_trait]
pub trait SessionReplacer: Send + 'static {
    async fn create(&mut self) -> Result<ReplacerOutcome, Error>;
    async fn close(&mut self) -> ();
}

#[derive(Debug, Clone)]
pub struct CurrentInletStatus {
    pub route: Route,
    pub worker: Address,
    pub connection_status: ConnectionStatus,
}

#[derive(Debug, Clone)]
pub enum ReplacerOutputKind {
    Inlet(CurrentInletStatus),
    Relay(RemoteRelayInfo),
}

#[derive(Debug, Clone)]
pub struct ReplacerOutcome {
    pub ping_route: Route,
    pub kind: ReplacerOutputKind,
}

pub(super) struct InnerSessionReplacer {
    inner: Mutex<Box<dyn SessionReplacer>>,
}

impl InnerSessionReplacer {
    pub fn new(inner: impl SessionReplacer) -> Self {
        Self {
            inner: Mutex::new(Box::new(inner)),
        }
    }
}

impl InnerSessionReplacer {
    async fn create(&self) -> Result<ReplacerOutcome, Error> {
        self.inner.lock().await.create().await
    }

    pub async fn close(&self) {
        self.inner.lock().await.close().await
    }

    pub async fn recreate(&self) -> Result<ReplacerOutcome, Error> {
        self.close().await;
        self.create().await
    }
}

#[derive(Clone)]
pub struct Session {
    key: String,
    inner: Arc<std::sync::Mutex<InnerSession>>,
}

pub struct InnerSession {
    connection: ConnectionStatus,
    replacer: Arc<InnerSessionReplacer>,
    pings: Vec<Ping>,
    last_outcome: Option<ReplacerOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, CborLen, Serialize, Deserialize)]
pub enum ConnectionStatus {
    #[n(0)]
    Down,
    #[n(1)]
    Degraded,
    #[n(2)]
    Up,
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionStatus::Down => write!(f, "{}", color_error("DOWN")),
            ConnectionStatus::Degraded => write!(f, "{}", color_warn("DEGRADED")),
            ConnectionStatus::Up => write!(f, "{}", color_ok("UP")),
        }
    }
}

impl TryFrom<String> for ConnectionStatus {
    type Error = ApiError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let inner = self.inner.lock().unwrap();
        f.debug_struct("Session")
            .field("key", &self.key)
            .field("last_outcome", &inner.last_outcome)
            .field("status", &inner.connection)
            .field("pings", &inner.pings)
            .finish()
    }
}

impl Session {
    /// Create a new session
    ///
    /// # Arguments
    ///
    /// * `replacer` - A structure implementing replacer [`SessionReplacer`],
    ///          which is used to create and close sessions
    pub fn new(replacer: impl SessionReplacer) -> Self {
        Self {
            key: hex::encode(random::<[u8; 8]>()),
            inner: Arc::new(std::sync::Mutex::new(InnerSession {
                connection: ConnectionStatus::Down,
                replacer: Arc::new(InnerSessionReplacer::new(replacer)),
                pings: Vec::new(),
                last_outcome: None,
            })),
        }
    }

    pub fn key(&self) -> &str {
        self.key.as_str()
    }

    pub fn ping_route(&self) -> Option<Route> {
        let inner = self.inner.lock().unwrap();
        inner.last_outcome.as_ref().map(|o| o.ping_route.clone())
    }

    pub fn connection_status(&self) -> ConnectionStatus {
        let inner = self.inner.lock().unwrap();
        inner.connection
    }

    pub fn status(&self) -> Option<ReplacerOutcome> {
        let inner = self.inner.lock().unwrap();
        inner.last_outcome.clone()
    }

    pub fn degraded(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.connection = ConnectionStatus::Degraded;
        inner.last_outcome = None;
    }

    pub fn up(&self, replacer_outcome: ReplacerOutcome) {
        let mut inner = self.inner.lock().unwrap();
        inner.connection = ConnectionStatus::Up;
        inner.last_outcome = Some(replacer_outcome);
    }

    pub fn down(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.connection = ConnectionStatus::Down;
        inner.last_outcome = None;
    }

    pub async fn close(self) -> Result<(), Error> {
        let replacer = {
            let inner = self.inner.lock().unwrap();
            inner.replacer.clone()
        };
        replacer.close().await;
        let mut inner = self.inner.lock().unwrap();
        inner.connection = ConnectionStatus::Down;
        inner.last_outcome = None;
        Ok(())
    }

    pub(super) fn replacer(&self) -> Arc<InnerSessionReplacer> {
        let inner = self.inner.lock().unwrap();
        inner.replacer.clone()
    }

    pub fn pings(&self) -> Vec<Ping> {
        let inner = self.inner.lock().unwrap();
        inner.pings.clone()
    }

    pub fn add_ping(&self, p: Ping) {
        let mut inner = self.inner.lock().unwrap();
        inner.pings.push(p);
    }

    pub fn clear_pings(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.pings.clear()
    }
}

#[derive(Debug, Default, Copy, Clone, Encode, Decode, CborLen, PartialEq, Eq)]
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
