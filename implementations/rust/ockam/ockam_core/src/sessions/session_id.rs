use crate::compat::rand::distributions::{Distribution, Standard};
use crate::compat::rand::Rng;
use crate::compat::string::{String, ToString};
use serde::{Deserialize, Serialize};

/// Unique random identifier of a session
#[derive(Clone, Eq, PartialEq, Debug, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Constructor
    pub fn new(str: &str) -> Self {
        Self(str.to_string())
    }
}

impl ToString for SessionId {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl Distribution<SessionId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SessionId {
        let address: [u8; 16] = rng.gen();
        SessionId(hex::encode(address))
    }
}
