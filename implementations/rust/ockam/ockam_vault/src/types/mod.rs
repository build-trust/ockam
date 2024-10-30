mod hashes;
mod public_keys;
mod secrets;
mod signatures;

pub use hashes::*;
pub use public_keys::*;
pub use secrets::*;
pub use signatures::*;

use alloc::format;
use alloc::string::String;

/// Returns a hex of 16bit representing the data, used for debugging.
fn debug_hash(data: &[u8]) -> String {
    let sum: u16 = data.iter().fold(0, |acc, x| acc.wrapping_add(*x as u16));
    format!("{:04x}", sum)
}
