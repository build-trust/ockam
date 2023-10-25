use ockam::identity::Identifier;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(super) struct Token {
    pub(super) attrs: HashMap<String, String>,
    pub(super) issued_by: Identifier,
    pub(super) created_at: Instant,
    pub(super) ttl: Duration,
    pub(super) ttl_count: u64,
}
