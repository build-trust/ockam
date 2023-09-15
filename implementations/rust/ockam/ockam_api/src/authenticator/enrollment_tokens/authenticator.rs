use lru::LruCache;
use ockam::identity::IdentityAttributesWriter;
use ockam_core::compat::sync::{Arc, RwLock};
use std::num::NonZeroUsize;
use std::time::Duration;

use crate::authenticator::enrollment_tokens::types::Token;
use crate::authenticator::enrollment_tokens::{EnrollmentTokenAcceptor, EnrollmentTokenIssuer};

pub(super) const MAX_TOKEN_DURATION: Duration = Duration::from_secs(600);

#[derive(Clone)]
pub struct EnrollmentTokenAuthenticator {
    pub(super) trust_context: String,
    // TODO: Replace with something sane and standard + implement expiration
    pub(super) tokens: Arc<RwLock<LruCache<[u8; 32], Token>>>,
}

impl EnrollmentTokenAuthenticator {
    pub fn new_worker_pair(
        trust_context: String,
        attributes_writer: Arc<dyn IdentityAttributesWriter>,
    ) -> (EnrollmentTokenIssuer, EnrollmentTokenAcceptor) {
        let base = Self {
            trust_context,
            tokens: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(128).expect("0 < 128"),
            ))),
        };
        (
            EnrollmentTokenIssuer(base.clone()),
            EnrollmentTokenAcceptor(base, attributes_writer),
        )
    }
}
