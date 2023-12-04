use std::process::Command;
use std::str;

use anyhow::anyhow;

use ockam::Result;
use ockam_api::authenticator::one_time_code::OneTimeCode;
use ockam_api::cli_state::enrollments::EnrollmentTicket;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Error;

/// Invoke the `ockam` command line in order to create a one-time token for
/// a given attribute name/value (and the default project on this machine)
pub async fn create_token(attribute_name: &str, attribute_value: &str) -> Result<OneTimeCode> {
    let token_output = Command::new("ockam")
        .args(vec![
            "project",
            "ticket",
            "--attribute",
            format!("{attribute_name}={attribute_value}").as_str(),
        ])
        .env_remove("OCKAM_LOG") // make sure that OCKAM_LOG is not set, otherwise the output will contain more than the token
        .output()
        .map_err(|e| error(format!("could not run the `ockam project ticket` successfully: {e:?}")))?;

    // we unwrap the result of decoding the token as UTF-8 since it should be some valid UTF-8 string
    let token_string = str::from_utf8(token_output.stdout.as_slice()).unwrap().trim();

    let decoded = hex::decode(token_string).map_err(|e| error(format!("{e}")))?;
    let ticket: EnrollmentTicket = serde_json::from_slice(&decoded).map_err(|e| error(format!("{e}")))?;

    Ok(ticket.one_time_code.clone())
}

fn error(message: String) -> Error {
    Error::new(Origin::Application, Kind::Invalid, anyhow!(message))
}
