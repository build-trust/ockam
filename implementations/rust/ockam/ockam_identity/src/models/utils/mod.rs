use minicbor::Decode;
use ockam_core::Result;

fn get_versioned_data<'a, T: Decode<'a, ()>>(data: &'a [u8]) -> Result<T> {
    Ok(minicbor::decode(data)?)
}

mod change_history;
mod credentials;
mod identifiers;
mod purpose_key_attestation;
mod timestamp;
