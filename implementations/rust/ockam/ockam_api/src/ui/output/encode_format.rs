use super::Output;
use crate::Result;
use clap::ValueEnum;
use miette::WrapErr;
use minicbor::{CborLen, Encode};

/// Data can be encoded in two formats:
///  - Plain: no encoding, the output is simply printed as a string
///  - Hex: the output is serialized using CBOR and the resulting bytes are represented as some HEX text
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
pub enum EncodeFormat {
    Plain,
    Hex,
}

impl EncodeFormat {
    pub fn encode_value<T>(&self, value: &T) -> Result<String>
    where
        T: Encode<()> + CborLen<()> + Output,
    {
        Ok(match self {
            EncodeFormat::Plain => value.item().wrap_err("Failed serialize output")?,
            EncodeFormat::Hex => {
                let bytes =
                    ockam_core::cbor_encode_preallocate(value).expect("Unable to encode response");
                hex::encode(bytes)
            }
        })
    }
}
