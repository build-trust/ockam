use ockam::identity::Identifier;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub(super) struct Token {
    pub(super) attrs: HashMap<String, String>,
    pub(super) generated_by: Identifier,
    pub(super) time: Instant,
    pub(super) max_token_duration: Duration,
}
