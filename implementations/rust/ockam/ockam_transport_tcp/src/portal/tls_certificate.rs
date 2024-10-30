use core::fmt::{Debug, Display, Formatter};
use minicbor::{Decode, Encode};
use ockam_core::async_trait;
use ockam_node::Context;
use serde::{Deserialize, Serialize};
use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::log::warn;

/// Refresh the certificate every day.
pub const DEFAULT_CACHE_RETENTION: Duration = Duration::from_secs(60 * 60 * 24);

/// When certificate refresh fails, retry every 12 hours.
pub const DEFAULT_RETRY_INTERVAL: Duration = Duration::from_secs(60 * 60 * 12);

/// Structure representing typical TLS certificates with the relative private key
/// to allow easy deployment
#[derive(Encode, Decode, Serialize, Deserialize, PartialEq, Debug, Clone)]
#[rustfmt::skip]
#[cbor(map)]
pub struct TlsCertificate {
    /// Public certificate chain, in PEM format.
    #[cbor(with = "minicbor::bytes")]
    #[n(1)] pub full_chain_pem: Vec<u8>,
    /// Private key, in PEM format.
    #[cbor(with = "minicbor::bytes")]
    #[n(2)] pub private_key_pem: Vec<u8>,
}

#[async_trait]
/// TLS certificate provider abstraction, to keep the implementation opaque
/// to the TCP transport.
pub trait TlsCertificateProvider: Send + Sync + Display + Debug + 'static {
    /// Returns a TLS certificate
    async fn get_certificate(&self, context: &Context) -> ockam_core::Result<TlsCertificate>;
}

/// This interface is used to make the testing simpler.
/// It's used as a static template.
trait Clock: Sync + Send + 'static {
    fn now(&self) -> Instant;
}

struct DefaultClock;

impl Clock for DefaultClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

struct CacheEntry {
    timestamp: Instant,
    last_retrieval_failure: Option<Instant>,
    certificate: TlsCertificate,
}

/// Creates a cache for [`TlsCertificateProvider`].
/// The cache will keep the certificate up to [`DEFAULT_CACHE_RETENTION`], afterward it will try
/// to refresh the certificate, but it'll return the previous one in case of failure.
pub fn new_certificate_provider_cache(
    certificate_provider: Arc<dyn TlsCertificateProvider>,
) -> Arc<dyn TlsCertificateProvider> {
    TlsCertificateCache::new_extended(
        certificate_provider,
        DEFAULT_CACHE_RETENTION,
        DEFAULT_RETRY_INTERVAL,
        DefaultClock {},
    )
}

struct TlsCertificateCache<T: Clock> {
    last_certificate: Arc<Mutex<Option<CacheEntry>>>,
    certificate_provider: Arc<dyn TlsCertificateProvider>,
    cache_retention: Duration,
    retry_interval: Duration,
    clock: T,
}

impl<T: Clock> TlsCertificateCache<T> {
    pub fn new_extended(
        certificate_provider: Arc<dyn TlsCertificateProvider>,
        cache_retention: Duration,
        retry_interval: Duration,
        clock: T,
    ) -> Arc<dyn TlsCertificateProvider> {
        Arc::new(Self {
            last_certificate: Default::default(),
            certificate_provider,
            cache_retention,
            retry_interval,
            clock,
        })
    }
}

impl<T: Clock> Display for TlsCertificateCache<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt(&self.certificate_provider, f)
    }
}

impl<T: Clock> Debug for TlsCertificateCache<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TlsCertificateCache")
            .field("cache_retention", &self.cache_retention)
            .field("retry_interval", &self.retry_interval)
            .field("certificate_provider", &self.certificate_provider)
            .finish()
    }
}

#[async_trait]
impl<T: Clock> TlsCertificateProvider for TlsCertificateCache<T> {
    async fn get_certificate(&self, context: &Context) -> ockam_core::Result<TlsCertificate> {
        let mut guard = self.last_certificate.lock().await;

        let now = self.clock.now();
        if let Some(entry) = guard.as_ref() {
            if now.sub(entry.timestamp) < self.cache_retention {
                return Ok(entry.certificate.clone());
            }

            // if recently failed, use the cache until the retry_interval is elapsed
            if let Some(last_retrieval_failure) = entry.last_retrieval_failure {
                if now.sub(last_retrieval_failure) < self.retry_interval {
                    return Ok(entry.certificate.clone());
                }
            }
        }

        let certificate = match self.certificate_provider.get_certificate(context).await {
            Ok(certificate) => {
                *guard = Some(CacheEntry {
                    timestamp: now,
                    last_retrieval_failure: None,
                    certificate: certificate.clone(),
                });
                certificate
            }
            Err(error) => {
                // At this point, the cache retention is expired but certificate refresh failed,
                // to avoid disruption the code returns the previous certificate.
                // We attempt certificate refresh again only after `retry_interval` is elapsed.
                if let Some(entry) = guard.as_mut() {
                    warn!("Cannot refresh the certificate: {error}. Reusing previous one.");
                    entry.last_retrieval_failure = Some(now);
                    entry.certificate.clone()
                } else {
                    return Err(error);
                }
            }
        };

        Ok(certificate)
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use ockam_core::errcode::{Kind, Origin};
    use ockam_node::Context;
    use std::ops::AddAssign;
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::Arc;
    use tokio::time::Duration;

    #[derive(Debug)]
    struct TestCertificateProvider {
        counter: Arc<AtomicU8>,
        return_certificate: Arc<AtomicBool>,
    }

    impl Display for TestCertificateProvider {
        fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
            write!(f, "TestCertificateProvider")
        }
    }

    #[async_trait]
    impl TlsCertificateProvider for TestCertificateProvider {
        async fn get_certificate(&self, _context: &Context) -> ockam_core::Result<TlsCertificate> {
            let counter = self.counter.fetch_add(1, Ordering::Relaxed);
            if self.return_certificate.load(Ordering::Relaxed) {
                Ok(TlsCertificate {
                    full_chain_pem: format!("test-{counter}").into_bytes(),
                    private_key_pem: format!("test-{counter}").into_bytes(),
                })
            } else {
                Err(ockam_core::Error::new(
                    Origin::Transport,
                    Kind::Timeout,
                    "timeout",
                ))
            }
        }
    }

    struct TestClock {
        now: Arc<std::sync::Mutex<Instant>>,
    }

    impl Clock for TestClock {
        fn now(&self) -> Instant {
            *self.now.lock().unwrap()
        }
    }

    #[ockam_macros::test]
    async fn test_tls_certificate(context: &mut Context) -> ockam_core::Result<()> {
        let return_certificate = Arc::new(AtomicBool::new(true));
        let get_certificate_counter = Arc::new(AtomicU8::new(0));
        let certificate_provider = Arc::new(TestCertificateProvider {
            counter: get_certificate_counter.clone(),
            return_certificate: return_certificate.clone(),
        });

        let now = Arc::new(std::sync::Mutex::new(Instant::now()));

        // creates a cache of 10 minutes, with 1 minute retry upon failure
        let cache = TlsCertificateCache::new_extended(
            certificate_provider,
            Duration::from_secs(60 * 10),
            Duration::from_secs(60),
            TestClock { now: now.clone() },
        );

        assert_eq!(get_certificate_counter.load(Ordering::Relaxed), 0);

        let certificate = cache.get_certificate(context).await.unwrap();
        assert_eq!(certificate.full_chain_pem, b"test-0");
        assert_eq!(certificate.private_key_pem, b"test-0");
        assert_eq!(get_certificate_counter.load(Ordering::Relaxed), 1);

        // advance the time by 9 minutes and 59 seconds, should return the previous certificate
        now.lock()
            .unwrap()
            .add_assign(Duration::from_secs(60 * 9 + 59));
        let certificate = cache.get_certificate(context).await.unwrap();
        assert_eq!(certificate.full_chain_pem, b"test-0");
        assert_eq!(certificate.private_key_pem, b"test-0");
        assert_eq!(get_certificate_counter.load(Ordering::Relaxed), 1);

        // 1 more second, and the certificate should be refreshed
        now.lock().unwrap().add_assign(Duration::from_secs(1));
        let certificate = cache.get_certificate(context).await.unwrap();
        assert_eq!(certificate.full_chain_pem, b"test-1");
        assert_eq!(certificate.private_key_pem, b"test-1");
        assert_eq!(get_certificate_counter.load(Ordering::Relaxed), 2);

        // 1 more second, and the certificate should be refreshed
        now.lock().unwrap().add_assign(Duration::from_secs(1));
        let certificate = cache.get_certificate(context).await.unwrap();
        assert_eq!(certificate.full_chain_pem, b"test-1");
        assert_eq!(certificate.private_key_pem, b"test-1");
        assert_eq!(get_certificate_counter.load(Ordering::Relaxed), 2);

        // advance the time by 10 minutes, this time the certificate provider will fail
        now.lock().unwrap().add_assign(Duration::from_secs(60 * 10));
        return_certificate.store(false, Ordering::Relaxed);
        let certificate = cache.get_certificate(context).await.unwrap();
        assert_eq!(certificate.full_chain_pem, b"test-1");
        assert_eq!(certificate.private_key_pem, b"test-1");
        assert_eq!(get_certificate_counter.load(Ordering::Relaxed), 3);

        // advance the time by 59 seconds, the old certificate is returned without another
        // refresh attempt
        now.lock().unwrap().add_assign(Duration::from_secs(59));
        let certificate = cache.get_certificate(context).await.unwrap();
        assert_eq!(certificate.full_chain_pem, b"test-1");
        assert_eq!(certificate.private_key_pem, b"test-1");
        assert_eq!(get_certificate_counter.load(Ordering::Relaxed), 3);

        // advance the time by 1 second, another failed refresh attempt
        now.lock().unwrap().add_assign(Duration::from_secs(1));
        let certificate = cache.get_certificate(context).await.unwrap();
        assert_eq!(certificate.full_chain_pem, b"test-1");
        assert_eq!(certificate.private_key_pem, b"test-1");
        assert_eq!(get_certificate_counter.load(Ordering::Relaxed), 4);

        // advance the time by 60 second, a successful refresh
        now.lock().unwrap().add_assign(Duration::from_secs(60));
        return_certificate.store(true, Ordering::Relaxed);
        let certificate = cache.get_certificate(context).await.unwrap();
        assert_eq!(certificate.full_chain_pem, b"test-4");
        assert_eq!(certificate.private_key_pem, b"test-4");
        assert_eq!(get_certificate_counter.load(Ordering::Relaxed), 5);

        // advance 9 minutes and 59 seconds, same certificate is returned
        now.lock()
            .unwrap()
            .add_assign(Duration::from_secs(9 * 60 + 59));
        return_certificate.store(true, Ordering::Relaxed);
        let certificate = cache.get_certificate(context).await.unwrap();
        assert_eq!(certificate.full_chain_pem, b"test-4");
        assert_eq!(certificate.private_key_pem, b"test-4");
        assert_eq!(get_certificate_counter.load(Ordering::Relaxed), 5);

        Ok(())
    }
}
