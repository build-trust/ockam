use ockam_core::compat::string::String;

#[cfg(feature = "unsafe_random")]
use ockam_core::compat::rand::{thread_rng, Rng};
#[cfg(not(feature = "unsafe_random"))]
use rand::{thread_rng, Rng};

/// A simple generator for unique, human-readable identifiers suitable
/// for use in distributed systems.
pub struct Unique;

impl Unique {
    /// Generate a short, human-readable 32-bit random identifier with
    /// the given prefix.
    pub fn with_prefix<S>(prefix: S) -> String
    where
        S: Into<String>,
    {
        let mut rng = thread_rng();
        const HEX: &[u8] = b"0123456789abcdef";

        let name: String = (0..8)
            .map(|_| {
                let idx: usize = rng.gen_range(0..HEX.len());
                HEX[idx] as char
            })
            .collect();

        format!("{}_{}", prefix.into(), name)
    }
}
