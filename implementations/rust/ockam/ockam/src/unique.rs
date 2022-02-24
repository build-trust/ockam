use ockam_core::compat::rand::{thread_rng, Rng};
use ockam_core::compat::string::String;

/// A generator for unique, human-readable identifiers suitable for use in
/// distributed systems.
///
/// Generates a short, human-readable 32-bit random identifier with the given
/// prefix.
// TODO: this probably should be moved into a `utils` module, or, honestly, into
// the kafka examples, which is currently the only place it's used.
pub fn unique_with_prefix(prefix: impl AsRef<str>) -> String {
    let mut rng = thread_rng();
    const HEX: &[u8] = b"0123456789abcdef";

    let name: String = (0..8)
        .map(|_| {
            let idx: usize = rng.gen_range(0..HEX.len());
            HEX[idx] as char
        })
        .collect();

    format!("{}_{}", prefix.as_ref(), name)
}
