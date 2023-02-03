use anyhow::anyhow;
use minicbor::bytes::ByteArray;
use minicbor::{Decode, Encode};
use ockam::Result;
use ockam_core::compat::rand;
use ockam_core::compat::rand::RngCore;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use std::process::Command;
use std::str;

/// Invoke the `ockam` command line in order to create a one-time token for
/// a given attribute name/value (and the default project on this machine)
pub async fn create_token(attribute_name: &str, attribute_value: &str) -> Result<OneTimeCode> {
    let token_output = Command::new("ockam")
        .args(vec![
            "project",
            "enroll",
            "--attribute",
            format!("{attribute_name}={attribute_value}").as_str(),
        ])
        .env_remove("OCKAM_LOG") // make sure that OCKAM_LOG is not set, otherwise the output will contain more than the token
        .output()
        .map_err(|e| error(format!("could not run the `ockam project enroll` successfully: {e:?}")))?;

    // we unwrap the result of decoding the token as UTF-8 since it should be some valid UTF-8 string
    let token_string = str::from_utf8(token_output.stdout.as_slice()).unwrap().trim();
    otc_parser(token_string)
}

/// Parse the token created by the `ockam project enroll --attribute attribute_name=attribute_value` command
pub fn otc_parser(val: &str) -> Result<OneTimeCode> {
    let bytes = hex::decode(val).map_err(|e| error(format!("{e}")))?;
    let code = <[u8; 32]>::try_from(bytes.as_slice()).map_err(|e| error(format!("{e}")))?;
    Ok(code.into())
}

/// A one-time code to enroll a member.
#[derive(Debug, Clone, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct OneTimeCode {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5112299>,
    #[n(1)] code: ByteArray<32>,
}

impl OneTimeCode {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut code = [0; 32];
        rand::thread_rng().fill_bytes(&mut code);
        OneTimeCode::from(code)
    }

    pub fn code(&self) -> &[u8; 32] {
        &self.code
    }
}

impl From<[u8; 32]> for OneTimeCode {
    fn from(code: [u8; 32]) -> Self {
        OneTimeCode {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            code: code.into(),
        }
    }
}

fn error(message: String) -> Error {
    Error::new(Origin::Application, Kind::Invalid, anyhow!(message))
}
