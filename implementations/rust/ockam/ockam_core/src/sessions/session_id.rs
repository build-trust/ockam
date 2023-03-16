use crate::compat::rand::distributions::{Distribution, Standard};
use crate::compat::rand::Rng;
use crate::compat::string::String;
use serde::{Deserialize, Serialize};

/// Unique random identifier of a session
#[derive(Clone, Eq, PartialEq, Debug, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SessionId(String);

impl Distribution<SessionId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SessionId {
        let address: [u8; 16] = rng.gen();
        SessionId(hex::encode(address))
    }
}
