use anyhow::anyhow;
use ockam::identity::credential::OneTimeCode;
use ockam::Result;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;
use std::process::Command;
use std::str;
use std::str::FromStr;

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
    OneTimeCode::from_str(token_string)
}

fn error(message: String) -> Error {
    Error::new(Origin::Application, Kind::Invalid, anyhow!(message))
}
