pub use cow_bytes::*;
pub use cow_str::*;

mod cow_bytes;
mod cow_str;
pub(crate) mod schema;

use crate::compat::vec::Vec;
use crate::Result;
use minicbor::{CborLen, Encode, Encoder};

/// Encode a type implementing [`Encode`] and return the encoded byte vector.
///
/// Pre-allocates memory beforehand by first calculating the resulting length.
#[cfg(feature = "alloc")]
pub fn cbor_encode_preallocate<T>(x: T) -> Result<Vec<u8>>
where
    T: Encode<()> + CborLen<()>,
{
    let output_len = minicbor::len(&x);
    let output = Vec::with_capacity(output_len);
    let mut e = Encoder::new(output);
    x.encode(&mut e, &mut ())?;
    Ok(e.into_writer())
}
