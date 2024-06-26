pub use cow_bytes::*;
pub use cow_str::*;

mod cow_bytes;
mod cow_str;
pub(crate) mod schema;

use crate::compat::vec::Vec;
use crate::Result;
use minicbor::{CborLen, Encode};

/// Encode a type implementing [`Encode`] and return the encoded byte vector.
///
/// Pre-allocates memory beforehand by first calculating the resulting length.
#[cfg(feature = "alloc")]
pub fn cbor_encode_preallocate<T>(x: T) -> Result<Vec<u8>>
where
    T: Encode<()> + CborLen<()>,
{
    // Due to minicbor bug this value is bigger than should be for structures having
    // #[cbor(transparent)] attribute.
    let max_expected_len = minicbor::len(&x);
    let mut output = Vec::with_capacity(max_expected_len);
    minicbor::encode(x, &mut output)?;
    Ok(output)
}
