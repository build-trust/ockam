use crate::compat::sync::Mutex;
use crate::{
    Address, IncomingAccessControl, LocalInfo, OutgoingAccessControl, RelayMessage, Route,
};
use alloc::vec::Vec;
use async_trait::async_trait;
use core::fmt::Debug;
use std::time::Instant;

const CACHE_MAX_SIZE: usize = 10;
const CACHE_DURATION_SECS: u64 = 1;

#[derive(Debug)]
struct CacheEntry {
    source: Address,
    destination: Address,
    onward_route: Route,
    return_route: Route,
    local_info: Vec<LocalInfo>,
    timestamp: Instant,
}

impl CacheEntry {
    fn from(relay_message: &RelayMessage) -> Self {
        Self {
            source: relay_message.source().clone(),
            destination: relay_message.destination().clone(),
            onward_route: relay_message.onward_route().clone(),
            return_route: relay_message.return_route().clone(),
            local_info: relay_message.local_message().local_info().to_vec(),
            timestamp: Instant::now(),
        }
    }

    /// Returns true if the cache entry is expired.
    fn is_expired(&self) -> bool {
        self.timestamp.elapsed().as_secs() >= CACHE_DURATION_SECS
    }

    /// Returns true if the relay message matches the cache entry.
    /// Everything except the payload is compared.
    fn matches(&self, relay_message: &RelayMessage) -> bool {
        self.source == *relay_message.source()
            && self.destination == *relay_message.destination()
            && self.onward_route == *relay_message.onward_route()
            && self.return_route == *relay_message.return_route()
            && self.local_info == relay_message.local_message().local_info()
    }
}

#[derive(Debug)]
struct Cache {
    cache: Mutex<Vec<CacheEntry>>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(Vec::new()),
        }
    }

    /// Returns true if the relay message is in the cache and not expired.
    pub fn exist_in_cache(&self, relay_message: &RelayMessage) -> bool {
        let mut cache_guard = self.cache.lock().unwrap();
        cache_guard
            .iter()
            .position(|entry| entry.matches(relay_message))
            .map(|position| {
                if cache_guard[position].is_expired() {
                    cache_guard.remove(position);
                    false
                } else {
                    true
                }
            })
            .unwrap_or(false)
    }

    /// Adds the relay message to the cache.
    pub fn add_authorized(&self, relay_message: &RelayMessage) {
        let mut cache_guard = self.cache.lock().unwrap();
        let position = cache_guard
            .iter()
            .position(|entry| entry.matches(relay_message));
        if let Some(position) = position {
            cache_guard.remove(position);
        }
        cache_guard.push(CacheEntry::from(relay_message));
        if cache_guard.len() > CACHE_MAX_SIZE {
            cache_guard.remove(0);
        }
    }
}

/// A wrapper for an incoming access control that caches successful authorizations.
/// The message is considered the same if everything except the payload is the same.
/// Keeps a cache of the last [`CACHE_MAX_SIZE`] authorized messages with validity of
/// [`CACHE_DURATION_SECS`] seconds.
#[derive(Debug)]
pub struct CachedIncomingAccessControl {
    cache: Cache,
    access_control: Box<dyn IncomingAccessControl>,
}

impl CachedIncomingAccessControl {
    /// Wraps an incoming access control with a cache.
    pub fn new(access_control: Box<dyn IncomingAccessControl>) -> Self {
        Self {
            cache: Cache::new(),
            access_control,
        }
    }
}

#[async_trait]
impl IncomingAccessControl for CachedIncomingAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> crate::Result<bool> {
        if self.cache.exist_in_cache(relay_msg) {
            return crate::allow();
        }
        let is_authorized = self.access_control.is_authorized(relay_msg).await?;
        if is_authorized {
            self.cache.add_authorized(relay_msg);
            crate::allow()
        } else {
            crate::deny()
        }
    }
}

/// A wrapper for an outgoing access control that caches successful authorizations.
/// The message is considered the same if everything except the payload is the same.
/// Keeps a cache of the last [`CACHE_MAX_SIZE`] authorized messages with validity of
/// [`CACHE_DURATION_SECS`] seconds.
#[derive(Debug)]
pub struct CachedOutgoingAccessControl {
    cache: Cache,
    access_control: Box<dyn OutgoingAccessControl>,
}

impl CachedOutgoingAccessControl {
    /// Wraps an outgoing access control with a cache.
    pub fn new(access_control: Box<dyn OutgoingAccessControl>) -> Self {
        Self {
            cache: Cache::new(),
            access_control,
        }
    }
}

#[async_trait]
impl OutgoingAccessControl for CachedOutgoingAccessControl {
    async fn is_authorized(&self, relay_msg: &RelayMessage) -> crate::Result<bool> {
        if self.cache.exist_in_cache(relay_msg) {
            return crate::allow();
        }
        let is_authorized = self.access_control.is_authorized(relay_msg).await?;
        if is_authorized {
            self.cache.add_authorized(relay_msg);
            crate::allow()
        } else {
            crate::deny()
        }
    }
}

#[cfg(test)]
#[allow(missing_docs)]
pub mod test {
    use crate::access_control::cache::{CacheEntry, CACHE_DURATION_SECS};
    use crate::{
        route, Address, IncomingAccessControl, LocalInfo, OutgoingAccessControl, RelayMessage,
    };
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use std::time::Instant;
    use tokio::time::sleep;

    #[derive(Debug)]
    struct DebugAccessControl {
        authorized: Arc<AtomicBool>,
    }

    #[async_trait]
    impl IncomingAccessControl for DebugAccessControl {
        async fn is_authorized(&self, _relay_msg: &RelayMessage) -> crate::Result<bool> {
            Ok(self.authorized.load(Ordering::Relaxed))
        }
    }

    #[async_trait]
    impl OutgoingAccessControl for DebugAccessControl {
        async fn is_authorized(&self, _relay_msg: &RelayMessage) -> crate::Result<bool> {
            Ok(self.authorized.load(Ordering::Relaxed))
        }
    }
    fn relay_message() -> RelayMessage {
        RelayMessage::new(
            Address::random_local(),
            Address::random_local(),
            crate::LocalMessage::new()
                .with_onward_route(route!["onward"])
                .with_return_route(route!["return"])
                .with_local_info(vec![LocalInfo::new("type".into(), vec![1, 2, 3])]),
        )
    }

    // deduplicated test for incoming and outgoing access control
    macro_rules! access_policy_test {
        ($struct_name:tt) => {
            let authorized = Arc::new(AtomicBool::new(false));
            let access_control = DebugAccessControl {
                authorized: authorized.clone(),
            };

            let access_control = crate::$struct_name::new(Box::new(access_control));
            let relay_msg = relay_message();

            // negative result is not cached
            assert!(!access_control.is_authorized(&relay_msg).await.unwrap());
            authorized.store(true, Ordering::Relaxed);
            assert!(access_control.is_authorized(&relay_msg).await.unwrap());

            // positive result is cached
            authorized.store(false, Ordering::Relaxed);
            assert!(access_control.is_authorized(&relay_msg).await.unwrap());

            // but it expires
            sleep(Duration::from_millis(CACHE_DURATION_SECS * 1000 + 100)).await;
            assert!(!access_control.is_authorized(&relay_msg).await.unwrap());

            // positive result is cached again until the cache is full
            authorized.store(true, Ordering::Relaxed);
            assert!(access_control.is_authorized(&relay_msg).await.unwrap());
            for _ in 0..crate::access_control::cache::CACHE_MAX_SIZE {
                let different_relay_msg = relay_message();
                assert!(access_control
                    .is_authorized(&different_relay_msg)
                    .await
                    .unwrap());
            }
            // the relay message is no longer cached
            authorized.store(false, Ordering::Relaxed);
            assert!(!access_control.is_authorized(&relay_msg).await.unwrap());
        };
    }

    #[tokio::test]
    pub async fn incoming_access_control() {
        access_policy_test!(CachedIncomingAccessControl);
    }

    #[tokio::test]
    pub async fn outgoing_access_control() {
        access_policy_test!(CachedOutgoingAccessControl);
    }

    #[test]
    pub fn cache_entry_matches() {
        let relay_msg = relay_message();

        // self matches
        let entry = CacheEntry::from(&relay_msg);
        assert!(entry.matches(&relay_msg));

        // payload is ignored
        let cloned = RelayMessage::new(
            relay_msg.source().clone(),
            relay_msg.destination().clone(),
            relay_msg.local_message().clone().with_payload(vec![1]),
        );
        assert!(entry.matches(&cloned));

        // we check that if any field is different, the entry does not match

        // source
        let cloned = RelayMessage::new(
            Address::random_local(),
            relay_msg.destination().clone(),
            relay_msg.local_message().clone(),
        );
        assert!(!entry.matches(&cloned));

        // destination
        let cloned = RelayMessage::new(
            relay_msg.source().clone(),
            Address::random_local(),
            relay_msg.local_message().clone(),
        );
        assert!(!entry.matches(&cloned));

        // onward route
        let cloned = RelayMessage::new(
            relay_msg.source().clone(),
            relay_msg.destination().clone(),
            relay_msg
                .local_message()
                .clone()
                .with_onward_route(route!["different"]),
        );
        assert!(!entry.matches(&cloned));

        // return route
        let cloned = RelayMessage::new(
            relay_msg.source().clone(),
            relay_msg.destination().clone(),
            relay_msg
                .local_message()
                .clone()
                .with_return_route(route!["different"]),
        );
        assert!(!entry.matches(&cloned));

        // local info
        let cloned = RelayMessage::new(
            relay_msg.source().clone(),
            relay_msg.destination().clone(),
            relay_msg
                .local_message()
                .clone()
                .with_local_info(vec![LocalInfo::new("type".into(), vec![1, 2, 3, 4])]),
        );
        assert!(!entry.matches(&cloned));
    }

    #[test]
    pub fn cache_entry_is_expired() {
        let entry = CacheEntry {
            source: Address::random_local(),
            destination: Address::random_local(),
            onward_route: route!["onward"],
            return_route: route!["return"],
            local_info: vec![],
            timestamp: Instant::now(),
        };

        // not expired
        assert!(!entry.is_expired());

        // expired
        let entry = CacheEntry {
            timestamp: Instant::now() - Duration::from_secs(CACHE_DURATION_SECS),
            ..entry
        };
        assert!(entry.is_expired());
    }
}
